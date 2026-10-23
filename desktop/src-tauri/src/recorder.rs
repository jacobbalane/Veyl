//! Only an explicit Start command permits filtered audio to enter the disk worker.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs::{self, File},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, AtomicU8, Ordering},
        mpsc::{self, SyncSender, TrySendError},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime, PrimitiveDateTime};
use tokio::sync::{oneshot, watch};
use veyl_link::{
    audio::AudioBlock,
    messages::{AudioConfig, Capture},
};

type Reply = oneshot::Sender<Result<RecordingInfo, String>>;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingInfo {
    pub id: String,
    pub seconds: u32,
    pub mark_pokes: bool,
    pub state: String,
    pub remaining_seconds: u32,
    pub blocks: [u64; 2],
    pub error: Option<String>,
}
enum Message {
    SourceInfo(Value, Value),
    Config(u64, AudioConfig),
    Status(u64, bool),
    Audio(u64, u64, AudioBlock),
    Start(u32, bool, Value, Reply),
    Stop(Reply),
    Mark,
}
struct View {
    current: Mutex<Option<RecordingInfo>>,
    updates: watch::Sender<Option<RecordingInfo>>,
}
impl View {
    fn publish(&self, info: RecordingInfo) {
        *self.current.lock().unwrap() = Some(info.clone());
        self.updates.send_replace(Some(info));
    }
}
#[derive(Clone)]
pub struct Recorder {
    root: PathBuf,
    sender: SyncSender<Message>,
    active: Arc<AtomicU64>,
    overflow: Arc<AtomicU8>,
    view: Arc<View>,
}
impl Recorder {
    pub fn new(root: &Path, keep_days: u32) -> io::Result<Self> {
        fs::create_dir_all(root)?;
        let root = root.canonicalize()?;
        prune(&root, keep_days, OffsetDateTime::now_utc())?;
        let (sender, receiver) = mpsc::sync_channel(400);
        let active = Arc::new(AtomicU64::new(0));
        let overflow = Arc::new(AtomicU8::new(0));
        let (updates, _) = watch::channel(None);
        let view = Arc::new(View {
            current: Mutex::new(None),
            updates,
        });
        let worker = Self {
            root,
            sender,
            active,
            overflow,
            view,
        };
        let directory = worker.root.clone();
        let enabled = worker.active.clone();
        let failed = worker.overflow.clone();
        let output = worker.view.clone();
        std::thread::Builder::new()
            .name("Veyl-recorder".into())
            .spawn(move || {
                let mut format: Option<(u64, AudioConfig)> = None;
                let mut running = false;
                let mut recording: Option<Recording> = None;
                let mut epoch = 0_u64;
                let mut source_info: Option<(Value, Value)> = None;
                loop {
                    let overflow = failed.swap(0, Ordering::AcqRel);
                    if overflow != 0 {
                        finish(&mut recording, &enabled, &output, Some("queue_full"));
                        if overflow & 2 != 0 {
                            format = None;
                            running = false;
                        }
                    }
                    if recording
                        .as_ref()
                        .is_some_and(|recording| recording.deadline <= Instant::now())
                    {
                        finish(&mut recording, &enabled, &output, None);
                    }
                    let message = match receiver.recv_timeout(Duration::from_millis(50)) {
                        Ok(message) => message,
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            if let Some(recording) = recording.as_ref() {
                                output.publish(recording.info());
                            }
                            continue;
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                    // A producer can report overflow while the worker is waiting for a command.
                    // Check again before Stop can report a complete recording.
                    let overflow = failed.swap(0, Ordering::AcqRel);
                    if overflow != 0 {
                        finish(&mut recording, &enabled, &output, Some("queue_full"));
                        if overflow & 2 != 0 {
                            format = None;
                            running = false;
                        }
                    }
                    if recording
                        .as_ref()
                        .is_some_and(|recording| recording.deadline <= Instant::now())
                    {
                        finish(&mut recording, &enabled, &output, None);
                    }
                    match message {
                        Message::SourceInfo(device, versions) => {
                            source_info = Some((device, versions))
                        }
                        Message::Config(generation, config) => {
                            if format
                                .as_ref()
                                .is_none_or(|(current, _)| generation >= *current)
                            {
                                finish(&mut recording, &enabled, &output, None);
                                format = Some((generation, config));
                                running = false;
                            }
                        }
                        Message::Status(generation, next) => {
                            if format
                                .as_ref()
                                .is_some_and(|(current, _)| generation == *current)
                            {
                                running = next;
                                if !running {
                                    finish(&mut recording, &enabled, &output, None);
                                }
                            }
                        }
                        Message::Start(seconds, marks, device, reply) => {
                            let result = if recording.is_some() {
                                Err("already_recording".into())
                            } else if ![30, 60, 120].contains(&seconds) {
                                Err("invalid_duration".into())
                            } else if let Some((_, config)) = format.as_ref().filter(|_| running) {
                                let (device, versions) = source_info.clone().unwrap_or_else(|| {
                                    (device, json!({"desktop": env!("CARGO_PKG_VERSION")}))
                                });
                                match Recording::create(
                                    &directory, config, seconds, marks, device, versions,
                                ) {
                                    Ok(next) => {
                                        let info = next.info();
                                        output.publish(info.clone());
                                        recording = Some(next);
                                        epoch += 1;
                                        enabled.store(epoch, Ordering::Release);
                                        Ok(info)
                                    }
                                    Err(_) => Err("write_failed".into()),
                                }
                            } else {
                                Err("not_listening".into())
                            };
                            // A cancelled caller must never leave a new recording running unseen.
                            let started = result.is_ok();
                            if reply.send(result).is_err() && started {
                                finish(&mut recording, &enabled, &output, None);
                            }
                        }
                        Message::Stop(reply) => {
                            finish(&mut recording, &enabled, &output, None);
                            let result = output
                                .current
                                .lock()
                                .unwrap()
                                .clone()
                                .ok_or_else(|| "not_recording".into());
                            let _ = reply.send(result);
                        }
                        Message::Audio(ticket, generation, block) => {
                            if ticket != 0
                                && ticket == enabled.load(Ordering::Acquire)
                                && format
                                    .as_ref()
                                    .is_some_and(|(current, _)| generation == *current)
                            {
                                if let Some(current) = recording.as_mut() {
                                    match current.audio(&block) {
                                        Ok(done) => {
                                            output.publish(current.info());
                                            if done {
                                                finish(&mut recording, &enabled, &output, None);
                                            }
                                        }
                                        Err(_) => finish(
                                            &mut recording,
                                            &enabled,
                                            &output,
                                            Some("write_failed"),
                                        ),
                                    }
                                }
                            }
                        }
                        Message::Mark => {
                            if let Some(current) = recording.as_mut() {
                                if current.mark().is_err() {
                                    finish(&mut recording, &enabled, &output, Some("write_failed"));
                                }
                            }
                        }
                    }
                }
                finish(&mut recording, &enabled, &output, None);
            })?;
        Ok(worker)
    }
    fn ingest(&self, message: Message) {
        let mask = if matches!(&message, Message::Audio(..)) {
            1
        } else {
            3
        };
        if self.sender.try_send(message).is_err() {
            self.overflow.fetch_or(mask, Ordering::AcqRel);
        }
    }
    pub fn configure(&self, generation: u64, config: AudioConfig) {
        self.ingest(Message::Config(generation, config));
    }
    pub fn source_info(&self, device: Value, versions: Value) {
        self.ingest(Message::SourceInfo(device, versions));
    }
    pub fn status(&self, generation: u64, capture: &Capture) {
        self.ingest(Message::Status(generation, capture.state == "running"));
    }
    pub fn disconnected(&self, generation: u64) {
        self.ingest(Message::Status(generation, false));
    }
    pub fn audio(&self, generation: u64, block: &AudioBlock) {
        let ticket = self.active.load(Ordering::Acquire);
        if ticket != 0 {
            self.ingest(Message::Audio(ticket, generation, block.clone()));
        }
    }
    pub async fn start(
        &self,
        seconds: u32,
        marks: bool,
        device: Value,
    ) -> Result<RecordingInfo, String> {
        let (reply, result) = oneshot::channel();
        self.send(Message::Start(seconds, marks, device, reply))?;
        result.await.map_err(|_| "write_failed".to_string())?
    }
    pub async fn stop(&self) -> Result<RecordingInfo, String> {
        let (reply, result) = oneshot::channel();
        self.send(Message::Stop(reply))?;
        result.await.map_err(|_| "write_failed".to_string())?
    }
    pub fn mark(&self) -> Result<(), String> {
        self.send(Message::Mark)
    }
    fn send(&self, message: Message) -> Result<(), String> {
        self.sender.try_send(message).map_err(|error| match error {
            TrySendError::Full(_) => "queue_full".into(),
            TrySendError::Disconnected(_) => "write_failed".into(),
        })
    }
    pub fn snapshot(&self) -> Option<RecordingInfo> {
        self.view.current.lock().unwrap().clone()
    }
    pub fn subscribe(&self) -> watch::Receiver<Option<RecordingInfo>> {
        self.view.updates.subscribe()
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn list(&self) -> io::Result<Vec<RecordingInfo>> {
        let mut recordings = vec![];
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let id = entry.file_name().to_string_lossy().into_owned();
            if valid_id(&id) && managed_directory(&entry.path()) {
                if let Ok(meta) = read_meta(&entry.path()) {
                    if let Ok(mut info) =
                        serde_json::from_value::<RecordingInfo>(meta["recording"].clone())
                    {
                        info.id = id;
                        if info.state == "recording" {
                            info.state = "failed".into();
                            info.error = Some("interrupted".into());
                        }
                        recordings.push(info);
                    }
                }
            }
        }
        if let Some(current) = self.snapshot() {
            recordings.retain(|info| info.id != current.id);
            recordings.push(current);
        }
        recordings.sort_by(|a, b| b.id.cmp(&a.id));
        Ok(recordings)
    }
    pub fn delete(&self, id: &str) -> io::Result<()> {
        if self
            .snapshot()
            .is_some_and(|info| info.id == id && info.state == "recording")
        {
            return Err(io::Error::other("Recording is active."));
        }
        remove_managed(&self.root, id)?;
        let mut view = self.view.current.lock().unwrap();
        if view.as_ref().is_some_and(|info| info.id == id) {
            *view = None;
            self.view.updates.send_replace(None);
        }
        Ok(())
    }
}
fn finish(current: &mut Option<Recording>, active: &AtomicU64, view: &View, error: Option<&str>) {
    active.store(0, Ordering::Release);
    if let Some(recording) = current.take() {
        view.publish(recording.finish(error));
    }
}

struct Recording {
    directory: PathBuf,
    meta: Value,
    info: RecordingInfo,
    deadline: Instant,
    waves: [Option<hound::WavWriter<BufWriter<File>>>; 2],
    blocks: BufWriter<File>,
    labels: BufWriter<File>,
    events: BufWriter<File>,
    first: [Option<u64>; 2],
    next: [u64; 2],
    latest_mic: Option<u64>,
}
impl Recording {
    fn create(
        root: &Path,
        config: &AudioConfig,
        seconds: u32,
        marks: bool,
        device: Value,
        versions: Value,
    ) -> io::Result<Self> {
        let now = OffsetDateTime::now_utc();
        let id_format = time::format_description::parse_borrowed::<2>(
            "[year][month][day]-[hour][minute][second]",
        )
        .map_err(io::Error::other)?;
        let mut suffix = [0; 4];
        getrandom::fill(&mut suffix).map_err(io::Error::other)?;
        let id = format!(
            "{}-{:02x}{:02x}{:02x}{:02x}",
            now.format(&id_format).map_err(io::Error::other)?,
            suffix[0],
            suffix[1],
            suffix[2],
            suffix[3]
        );
        let directory = root.join(&id);
        fs::create_dir(&directory)?;
        let info = RecordingInfo {
            id,
            seconds,
            mark_pokes: marks,
            state: "recording".into(),
            remaining_seconds: seconds,
            blocks: [0, 0],
            error: None,
        };
        let meta = json!({
            "format": 1, "sample_rate": config.sample_rate, "ref_channels": config.ref_channels,
            "mic_channels": config.mic_channels, "source": config.source, "device": device,
            "app_versions": versions, "hp_cutoff_hz": config.hp_cutoff_hz,
            "started_at": now.format(&Rfc3339).map_err(io::Error::other)?,
            "ref_first_frame": 0, "mic_first_frame": 0, "notes": "", "recording": info,
        });
        // Publish metadata first so a partial file-creation failure is visible on restart.
        persist_meta(&directory, &meta)?;
        let wave = |name, channels| {
            hound::WavWriter::create(
                directory.join(name),
                hound::WavSpec {
                    channels,
                    sample_rate: config.sample_rate,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                },
            )
            .map_err(io::Error::other)
        };
        let recording = Self {
            waves: [
                Some(wave("ref.wav", config.ref_channels as u16)?),
                Some(wave("mic.wav", config.mic_channels as u16)?),
            ],
            blocks: BufWriter::new(File::create(directory.join("blocks.jsonl"))?),
            labels: BufWriter::new(File::create(directory.join("labels.jsonl"))?),
            events: BufWriter::new(File::create(directory.join("events.jsonl"))?),
            directory,
            meta,
            info,
            deadline: Instant::now() + Duration::from_secs(seconds as u64),
            first: [None, None],
            next: [0, 0],
            latest_mic: None,
        };
        recording.save_meta()?;
        Ok(recording)
    }
    fn info(&self) -> RecordingInfo {
        let mut info = self.info.clone();
        info.remaining_seconds = self
            .deadline
            .saturating_duration_since(Instant::now())
            .as_secs_f64()
            .ceil() as u32;
        info
    }
    fn save_meta(&self) -> io::Result<()> {
        let mut meta = self.meta.clone();
        meta["recording"] = serde_json::to_value(&self.info).map_err(io::Error::other)?;
        meta["ref_first_frame"] = json!(self.first[0].unwrap_or(0));
        meta["mic_first_frame"] = json!(self.first[1].unwrap_or(0));
        persist_meta(&self.directory, &meta)
    }
    fn audio(&mut self, block: &AudioBlock) -> io::Result<bool> {
        let index = block.stream as usize;
        if index > 1 {
            return Err(io::Error::other("Invalid recording stream."));
        }
        let rate = self.meta["sample_rate"].as_u64().unwrap() as u32;
        let channels = self.meta[if index == 0 {
            "ref_channels"
        } else {
            "mic_channels"
        }]
        .as_u64()
        .unwrap() as u8;
        if block.sample_rate != rate
            || block.channels != channels
            || block.frames as u32 != rate / 100
        {
            return Err(io::Error::other("Invalid filtered recording block."));
        }
        block.encode()?;
        if self.first[index].is_none() {
            self.first[index] = Some(block.first_frame);
            self.next[index] = block.first_frame;
            self.save_meta()?;
        }
        let start = self.first[index].unwrap();
        let end = block
            .first_frame
            .checked_add(block.frames as u64)
            .ok_or_else(|| io::Error::other("Recording frame overflow."))?;
        let limit = start
            .checked_add(rate as u64 * self.info.seconds as u64)
            .ok_or_else(|| io::Error::other("Recording frame overflow."))?;
        if block.first_frame < self.next[index] {
            return Err(io::Error::other("Recording frames went backwards."));
        }
        if end <= limit {
            let wave = self.waves[index].as_mut().unwrap();
            // Missing frames are silent placeholders; JSONL preserves which blocks existed.
            for _ in self.next[index]..block.first_frame {
                for _ in 0..channels {
                    wave.write_sample(0_i16).map_err(io::Error::other)?;
                }
            }
            for &sample in &block.samples {
                wave.write_sample(sample).map_err(io::Error::other)?;
            }
            serde_json::to_writer(&mut self.blocks, &json!({ "stream": block.stream, "seq": block.seq, "first_frame": block.first_frame,
                "capture_ns": block.capture_ns, "flags": block.flags, "pre_rms_dbfs": block.pre_rms_dbfs, "pre_peak": block.pre_peak })).map_err(io::Error::other)?;
            self.blocks.write_all(b"\n")?;
            self.info.blocks[index] += 1;
            self.next[index] = end;
            if index == 1 {
                self.latest_mic = Some(end - 1);
            }
        } else {
            self.next[index] = limit;
        }
        Ok((0..2).all(|stream| {
            self.first[stream].is_some_and(|first| {
                self.next[stream] >= first + rate as u64 * self.info.seconds as u64
            })
        }))
    }
    fn mark(&mut self) -> io::Result<()> {
        if self.info.mark_pokes {
            if let Some(frame) = self.latest_mic {
                serde_json::to_writer(
                    &mut self.labels,
                    &json!({"frame": frame, "kind": "poke_mark"}),
                )
                .map_err(io::Error::other)?;
                self.labels.write_all(b"\n")?;
            }
        }
        Ok(())
    }
    fn finish(mut self, error: Option<&str>) -> RecordingInfo {
        let mut failed = error.is_some();
        for wave in &mut self.waves {
            if let Some(wave) = wave.take() {
                failed |= wave.finalize().is_err();
            }
        }
        failed |= self.blocks.flush().is_err();
        failed |= self.labels.flush().is_err();
        failed |= self.events.flush().is_err();
        self.info.remaining_seconds = 0;
        self.info.state = if failed { "failed" } else { "saved" }.into();
        self.info.error = if failed {
            Some(error.unwrap_or("write_failed").into())
        } else {
            None
        };
        if self.save_meta().is_err() {
            self.info.state = "failed".into();
            self.info.error = Some("write_failed".into());
        }
        self.info.clone()
    }
}
fn valid_id(id: &str) -> bool {
    id.len() == 24
        && id.bytes().enumerate().all(|(index, byte)| match index {
            8 | 15 => byte == b'-',
            0..=14 => byte.is_ascii_digit(),
            _ => byte.is_ascii_hexdigit(),
        })
}
fn persist_meta(directory: &Path, meta: &Value) -> io::Result<()> {
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    serde_json::to_writer_pretty(temporary.as_file_mut(), meta).map_err(io::Error::other)?;
    temporary.as_file_mut().sync_all()?;
    temporary
        .persist(directory.join("meta.json"))
        .map_err(|error| error.error)?;
    Ok(())
}
fn managed_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| {
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                return false;
            }
        }
        meta.is_dir() && !meta.file_type().is_symlink()
    })
}
fn read_meta(directory: &Path) -> io::Result<Value> {
    use std::io::Read;
    let mut bytes = vec![];
    File::open(directory.join("meta.json"))?
        .take(1_048_577)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 1_048_576 {
        return Err(io::Error::other("Recording metadata is too large."));
    }
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}
fn remove_managed(root: &Path, id: &str) -> io::Result<()> {
    let path = root.join(id);
    if !valid_id(id) || !managed_directory(&path) || path.canonicalize()?.parent() != Some(root) {
        return Err(io::Error::other("Invalid recording folder."));
    }
    fs::remove_dir_all(path)
}
fn prune(root: &Path, days: u32, now: OffsetDateTime) -> io::Result<()> {
    if days == 0 {
        return Ok(());
    }
    let format =
        time::format_description::parse_borrowed::<2>("[year][month][day]-[hour][minute][second]")
            .map_err(io::Error::other)?;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let id = entry.file_name().to_string_lossy().into_owned();
        if valid_id(&id) && managed_directory(&entry.path()) {
            if let Ok(time) = PrimitiveDateTime::parse(&id[..15], &format) {
                if now - time.assume_utc() > time::Duration::days(days as i64) {
                    remove_managed(root, &id)?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> AudioConfig {
        AudioConfig {
            run_id: 1,
            source: "test_sound".into(),
            sample_rate: 48000,
            frames_per_block: 480,
            ref_channels: 2,
            mic_channels: 2,
            mic_source: "unprocessed".into(),
            effects_off: vec![],
            hp_cutoff_hz: 8000,
            mics: vec![],
            sound_check: None,
        }
    }
    fn capture() -> Capture {
        Capture {
            state: "running".into(),
            source: "test_sound".into(),
            route: "speaker".into(),
            reason: None,
        }
    }
    #[tokio::test]
    async fn consent_gaps_marks_and_epochs_round_trip_through_the_lab_reader() {
        let temporary = tempfile::tempdir().unwrap();
        let recorder = Recorder::new(temporary.path(), 7).unwrap();
        let input = veyl_lab::recording::Input::generated(veyl_sim::poke_basic(1, 0.1).unwrap());
        // Receiving audio before consent creates no audio files.
        recorder.audio(7, &input.block(0, 0).unwrap());
        assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 0);
        assert_eq!(
            recorder.start(30, true, json!({})).await.unwrap_err(),
            "not_listening"
        );
        recorder.configure(7, config());
        recorder.status(7, &capture());
        assert_eq!(
            recorder.start(31, true, json!({})).await.unwrap_err(),
            "invalid_duration"
        );
        let first = recorder
            .start(30, true, json!({"model":"Simulator"}))
            .await
            .unwrap();
        assert!(recorder.delete(&first.id).is_err());
        let epoch = recorder.active.load(Ordering::Acquire);
        let mut expected = vec![];
        for index in 0..input.blocks.len() {
            let mut block = input.block(index, 0).unwrap();
            block.first_frame += if block.stream == 0 { 1000 } else { 2000 };
            if block.seq == 3 {
                continue;
            }
            if block.seq == 4 {
                block.flags |= 1;
            }
            if block.seq == 0 {
                block.capture_ns = i64::MIN;
            }
            if block.seq == 5 && block.stream == 1 {
                recorder.mark().unwrap();
            }
            recorder.audio(7, &block);
            expected.push(block);
        }
        let saved = recorder.stop().await.unwrap();
        assert_eq!(saved.state, "saved");
        assert_eq!(saved.blocks, [9, 9]);
        let restored = veyl_lab::recording::Input::read(&temporary.path().join(&saved.id)).unwrap();
        assert_eq!(
            (restored.meta.ref_first_frame, restored.meta.mic_first_frame),
            (1000, 2000)
        );
        assert_eq!(restored.blocks.len(), 18);
        assert_eq!(restored.labels.len(), 1);
        assert_eq!(restored.labels[0].frame, 2000 + 5 * 480 - 1);
        assert!(restored.reference[3 * 960..4 * 960]
            .iter()
            .all(|&sample| sample == 0));
        for (index, expected) in expected.iter().enumerate() {
            let block = restored.block(index, 0).unwrap();
            assert!(block.samples == expected.samples);
            assert_eq!(
                (block.first_frame, block.flags, block.pre_rms_dbfs),
                (expected.first_frame, expected.flags, expected.pre_rms_dbfs)
            );
        }
        let summary = veyl_lab::analyze::statistics(&restored).unwrap();
        assert_eq!(summary.reference.missing_frames, 480);
        assert_eq!(summary.microphone.flagged_gaps, 1);
        let second = recorder.start(30, false, json!({})).await.unwrap();
        // A callback delayed across Stop/Start cannot write into the new consent period.
        recorder
            .sender
            .send(Message::Audio(epoch, 7, input.block(0, 0).unwrap()))
            .unwrap();
        recorder.audio(6, &input.block(0, 0).unwrap());
        for index in 0..2 {
            recorder.audio(7, &input.block(index, 0).unwrap());
        }
        recorder.mark().unwrap();
        recorder.disconnected(7);
        let second_saved = recorder.stop().await.unwrap();
        assert_eq!(second_saved.blocks, [1, 1]);
        assert!(
            veyl_lab::recording::Input::read(&temporary.path().join(&second.id))
                .unwrap()
                .labels
                .is_empty()
        );
        assert_eq!(recorder.list().unwrap().len(), 2);
        recorder.delete(&saved.id).unwrap();
        recorder.delete(&second.id).unwrap();
        assert!(recorder.list().unwrap().is_empty());
        for id in ["..", "../outside", "C:\\", "20260101-000000-00000000/../x"] {
            assert!(recorder.delete(id).is_err());
        }
    }
    #[tokio::test]
    async fn ordinary_capture_and_player_audio_do_not_need_the_test_sound_flag() {
        let temporary = tempfile::tempdir().unwrap();
        let recorder = Recorder::new(temporary.path(), 7).unwrap();
        let input = veyl_lab::recording::Input::generated(veyl_sim::poke_basic(1, 0.1).unwrap());
        for (generation, source) in [(1, "capture"), (2, "player")] {
            let mut configuration = config();
            configuration.source = source.into();
            let mut status = capture();
            status.source = source.into();
            recorder.configure(generation, configuration);
            recorder.status(generation, &status);
            recorder.start(30, false, json!({})).await.unwrap();
            for index in 0..input.blocks.len() {
                let mut block = input.block(index, 0).unwrap();
                block.flags &= !12;
                if source == "player" && block.stream == 0 {
                    block.flags |= 8;
                }
                recorder.audio(generation, &block);
            }
            let saved = recorder.stop().await.unwrap();
            assert_eq!(saved.state, "saved");
            assert_eq!(saved.blocks, [10, 10]);
            let restored =
                veyl_lab::recording::Input::read(&temporary.path().join(&saved.id)).unwrap();
            assert_eq!(restored.meta.source, source);
            for block in &restored.blocks {
                assert_eq!(block.flags & 4, 0);
                assert_eq!(
                    block.flags & 8 != 0,
                    source == "player" && block.stream == 0
                );
            }
        }
    }
    #[test]
    fn retention_removes_only_expired_managed_folders_and_zero_keeps_them() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        for id in [
            "20260101-000000-00000000",
            "20261021-000000-00000000",
            "my-files",
        ] {
            fs::create_dir(root.join(id)).unwrap();
        }
        let now = OffsetDateTime::parse("2026-10-22T00:00:00Z", &Rfc3339).unwrap();
        prune(&root, 0, now).unwrap();
        assert!(root.join("20260101-000000-00000000").exists());
        prune(&root, 7, now).unwrap();
        assert!(!root.join("20260101-000000-00000000").exists());
        assert!(root.join("20261021-000000-00000000").exists());
        assert!(root.join("my-files").exists());
    }
    #[tokio::test]
    async fn queue_failure_stops_recording_and_invalid_audio_is_never_reported_as_saved() {
        let temporary = tempfile::tempdir().unwrap();
        let recorder = Recorder::new(temporary.path(), 7).unwrap();
        recorder.configure(1, config());
        recorder.status(1, &capture());
        recorder.start(30, true, json!({})).await.unwrap();
        let input = veyl_lab::recording::Input::generated(veyl_sim::poke_basic(1, 0.01).unwrap());
        let mut block = input.block(0, 0).unwrap();
        block.frames = 479;
        recorder.audio(1, &block);
        let failed = recorder.stop().await.unwrap();
        assert_eq!(failed.state, "failed");
        assert_eq!(failed.blocks, [0, 0]);
        recorder.start(30, true, json!({})).await.unwrap();
        recorder.overflow.store(3, Ordering::Release);
        let failed = recorder.stop().await.unwrap();
        assert_eq!(failed.state, "failed");
        assert_eq!(failed.error.as_deref(), Some("queue_full"));
        assert_eq!(recorder.active.load(Ordering::Acquire), 0);
        assert_eq!(
            recorder.start(30, true, json!({})).await.unwrap_err(),
            "not_listening"
        );
    }
}
