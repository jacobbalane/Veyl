use super::*;
use crate::{
    pc_devices::AudioDevice,
    source::{PhoneSource, SoundSource},
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};
use veyl_link::messages::{AudioConfig, Capture};

enum Event {
    Prepared(u8, CaptureDetails),
    Started(u8),
    Ended(&'static str),
}
pub struct CaptureGroup {
    stop: Arc<AtomicBool>,
    go: Arc<AtomicBool>,
    events: mpsc::Receiver<Event>,
    finished: Vec<oneshot::Receiver<()>>,
    pub reference: Option<CaptureDetails>,
    pub microphone: Option<CaptureDetails>,
    source: PhoneSource,
    generation: u64,
}
impl CaptureGroup {
    pub async fn start(
        speaker: AudioDevice,
        microphone: AudioDevice,
        source: PhoneSource,
        generation: u64,
    ) -> io::Result<Self> {
        #[cfg(windows)]
        {
            Self::with_factory(
                speaker,
                microphone,
                source,
                generation,
                Arc::new(super::native::Wasapi::open),
            )
            .await
        }
        #[cfg(not(windows))]
        {
            let _ = (speaker, microphone, source, generation);
            Err(invalid("PC capture needs Windows."))
        }
    }
    pub async fn with_factory(
        speaker: AudioDevice,
        microphone: AudioDevice,
        source: PhoneSource,
        generation: u64,
        factory: CaptureFactory,
    ) -> io::Result<Self> {
        let (events, receiver) = mpsc::channel(8);
        let stop = Arc::new(AtomicBool::new(false));
        let go = Arc::new(AtomicBool::new(false));
        let mut group = Self {
            stop,
            go,
            events: receiver,
            finished: vec![],
            reference: None,
            microphone: None,
            source,
            generation,
        };
        for (stream, device) in [(0, speaker.clone()), (1, microphone.clone())] {
            let stop = group.stop.clone();
            let go = group.go.clone();
            let factory = factory.clone();
            let events = events.clone();
            let source = group.source.clone();
            let (finished, wait) = oneshot::channel();
            group.finished.push(wait);
            std::thread::Builder::new()
                .name(
                    if stream == 0 {
                        "Veyl-loopback"
                    } else {
                        "Veyl-microphone"
                    }
                    .into(),
                )
                .spawn(move || {
                    let result = capture_thread(
                        &device, stream, generation, &source, factory, &stop, &go, &events,
                    );
                    if let Err(error) = result {
                        stop.store(true, Ordering::Release);
                        #[cfg(windows)]
                        let code = super::native::failure_code(&error);
                        #[cfg(not(windows))]
                        let code = {
                            let _ = error;
                            "pc_audio_error"
                        };
                        let _ = events.blocking_send(Event::Ended(code));
                    }
                    let _ = finished.send(());
                })?;
        }
        drop(events);
        let initialized = tokio::time::timeout(Duration::from_secs(5), async {
            while group.reference.is_none() || group.microphone.is_none() {
                match group.events.recv().await {
                    Some(Event::Prepared(0, details)) => group.reference = Some(details),
                    Some(Event::Prepared(1, details)) => group.microphone = Some(details),
                    Some(Event::Ended(code)) => return Err(io::Error::other(code)),
                    _ => return Err(invalid("PC capture preparation ended.")),
                }
            }
            let reference = group.reference.as_ref().unwrap(); let mic = group.microphone.as_ref().unwrap();
            group.source.source_info(generation, serde_json::json!({"os":"windows", "speaker": speaker, "microphone": microphone, "reference":reference, "capture":mic}), serde_json::json!({"desktop":env!("CARGO_PKG_VERSION")}));
            group.source.configure(generation, AudioConfig {
                run_id: generation, source: "pc".into(), sample_rate: 48000, frames_per_block: 480,
                ref_channels: reference.format.output_channels(), mic_channels: mic.format.output_channels(),
                mic_source: if mic.raw_mode { "raw" } else { "shared" }.into(), effects_off: if mic.raw_mode { vec!["raw".into()] } else { vec![] },
                hp_cutoff_hz: 8000, mics: vec![], sound_check: None,
            }).await?;
            group.source.status(generation, Capture { state:"running".into(), source:"pc".into(), route:"speaker".into(), reason:None }).await?;
            group.go.store(true, Ordering::Release);
            let mut started = [false; 2];
            while started != [true, true] {
                match group.events.recv().await {
                    Some(Event::Started(stream)) if stream < 2 => started[stream as usize] = true,
                    Some(Event::Ended(code)) => return Err(io::Error::other(code)),
                    _ => return Err(invalid("PC capture did not start.")),
                }
            }
            Ok(())
        }).await.map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "PC capture initialization timed out.")).and_then(|result| result);
        if let Err(error) = initialized {
            group.shutdown().await;
            return Err(error);
        }
        Ok(group)
    }
    pub async fn next_failure(&mut self) -> &'static str {
        loop {
            match self.events.recv().await {
                Some(Event::Ended(code)) => return code,
                None => return "pc_audio_error",
                _ => {}
            }
        }
    }
    pub async fn shutdown(mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = tokio::time::timeout(Duration::from_secs(2), async {
            for finished in self.finished.drain(..) {
                let _ = finished.await;
            }
        })
        .await;
        let _ = self.source.disconnected(self.generation).await;
    }
}
impl Drop for CaptureGroup {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.source.disconnect_now(self.generation);
    }
}
#[allow(clippy::too_many_arguments)]
fn capture_thread(
    device: &AudioDevice,
    stream: u8,
    generation: u64,
    source: &PhoneSource,
    factory: CaptureFactory,
    stop: &AtomicBool,
    go: &AtomicBool,
    events: &mpsc::Sender<Event>,
) -> io::Result<()> {
    let mut capture = factory(device, stream)?;
    let details = capture.details();
    let mut processor = Processor::new(details.format.clone(), stream)?;
    events
        .blocking_send(Event::Prepared(stream, details))
        .map_err(|_| invalid("PC capture owner stopped."))?;
    while !go.load(Ordering::Acquire) {
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    if stop.load(Ordering::Acquire) {
        return Ok(());
    }
    capture.start()?;
    let result = (|| {
        events
            .blocking_send(Event::Started(stream))
            .map_err(|_| invalid("PC capture owner stopped."))?;
        while !stop.load(Ordering::Acquire) {
            capture.wait()?;
            if stop.load(Ordering::Acquire) {
                break;
            }
            capture.poll(&mut |packet| {
                processor.push(packet, &mut |block| {
                    source.audio(generation, block, None).map(|_| ())
                })
            })?;
        }
        Ok(())
    })();
    capture.stop();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{engine_host::EngineHost, recorder::Recorder};
    use std::sync::atomic::AtomicUsize;
    struct Fake {
        format: CaptureFormat,
        frame: u64,
        bytes: Vec<u8>,
        started: bool,
        starts: Arc<AtomicUsize>,
        stops: Arc<AtomicUsize>,
    }
    impl CaptureBackend for Fake {
        fn details(&self) -> CaptureDetails {
            CaptureDetails {
                format: self.format.clone(),
                raw_mode: true,
                event_driven: false,
                buffer_ms: 20.0,
                pro_audio_thread: false,
            }
        }
        fn start(&mut self) -> io::Result<()> {
            self.started = true;
            self.starts.fetch_add(1, Ordering::AcqRel);
            Ok(())
        }
        fn wait(&mut self) -> io::Result<()> {
            std::thread::sleep(Duration::from_millis(2));
            Ok(())
        }
        fn poll(
            &mut self,
            sink: &mut dyn FnMut(CapturePacket<'_>) -> io::Result<()>,
        ) -> io::Result<()> {
            let frames = self.format.sample_rate as usize / 50;
            sink(CapturePacket {
                bytes: &self.bytes,
                frames,
                device_position: Some(self.frame),
                capture_ns: Some(
                    1_000_000_000
                        + self.frame as i64 * 1_000_000_000 / self.format.sample_rate as i64,
                ),
                discontinuity: false,
                silent: false,
            })?;
            self.frame += frames as u64;
            Ok(())
        }
        fn stop(&mut self) {
            if self.started {
                self.started = false;
                self.stops.fetch_add(1, Ordering::AcqRel);
            }
        }
    }
    fn device(id: &str) -> AudioDevice {
        AudioDevice {
            id: id.into(),
            name: "Fake endpoint".into(),
            form_factor: "speakers".into(),
            raw_supported: true,
            bluetooth: false,
            built_in: true,
        }
    }
    fn factory(starts: Arc<AtomicUsize>, stops: Arc<AtomicUsize>) -> CaptureFactory {
        Arc::new(move |device, stream| {
            if device.id == "unavailable" {
                return Err(io::Error::other("Fake endpoint unavailable."));
            }
            let rate = if stream == 0 { 44100 } else { 48000 };
            let channels = if stream == 0 { 2 } else { 1 };
            let frames = rate / 50;
            let bytes = (0..frames)
                .flat_map(|frame| {
                    let value =
                        (std::f32::consts::TAU * 12000.0 * frame as f32 / rate as f32).sin() * 0.2;
                    (0..channels).flat_map(move |_| value.to_le_bytes())
                })
                .collect();
            Ok(Box::new(Fake {
                format: CaptureFormat {
                    sample_rate: rate,
                    channels,
                    encoding: Encoding::Float32,
                    frame_stride: channels as usize * 4,
                },
                frame: 0,
                bytes,
                started: false,
                starts: starts.clone(),
                stops: stops.clone(),
            }))
        })
    }
    #[tokio::test]
    async fn separate_capture_threads_filter_and_resample_before_the_engine_or_pc_recording() {
        tokio::time::timeout(Duration::from_secs(5), async {
            let temporary = tempfile::tempdir().unwrap();
            let recorder = Recorder::new(temporary.path(), 7).unwrap();
            let (host, source) = EngineHost::with_recorder(Some(recorder.clone()));
            let starts = Arc::new(AtomicUsize::new(0));
            let stops = Arc::new(AtomicUsize::new(0));
            let group = CaptureGroup::with_factory(
                device("speaker"),
                device("mic"),
                source,
                7,
                factory(starts.clone(), stops.clone()),
            )
            .await
            .unwrap();
            assert_eq!(starts.load(Ordering::Acquire), 2);
            assert_eq!(std::fs::read_dir(temporary.path()).unwrap().count(), 0);
            let active = recorder
                .start(30, true, serde_json::json!({}))
                .await
                .unwrap();
            while recorder
                .snapshot()
                .unwrap()
                .blocks
                .iter()
                .any(|&blocks| blocks < 10)
            {
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
            recorder.mark().unwrap();
            group.shutdown().await;
            let saved = recorder.stop().await.unwrap();
            assert_eq!(stops.load(Ordering::Acquire), 2);
            assert_eq!(saved.state, "saved");
            let input =
                veyl_lab::recording::Input::read(&temporary.path().join(active.id)).unwrap();
            assert_eq!(input.meta.source, "pc");
            assert_eq!((input.meta.ref_channels, input.meta.mic_channels), (2, 1));
            assert_eq!(input.meta.sample_rate, 48000);
            assert_eq!(input.labels.len(), 1);
            assert!(input.blocks.iter().all(|block| block.flags & 4 != 0));
            assert!(host.levels().arrival_ms.is_none());
            assert_eq!(host.levels().engine_drops, 0);
        })
        .await
        .unwrap();
    }
    #[tokio::test]
    async fn failed_preparation_does_not_start_either_audio_stream_and_closes_the_peer() {
        let (_, source) = EngineHost::start();
        let starts = Arc::new(AtomicUsize::new(0));
        let stops = Arc::new(AtomicUsize::new(0));
        let result = tokio::time::timeout(
            Duration::from_secs(3),
            CaptureGroup::with_factory(
                device("speaker"),
                device("unavailable"),
                source,
                1,
                factory(starts.clone(), stops.clone()),
            ),
        )
        .await
        .unwrap();
        assert!(result.is_err());
        assert_eq!(starts.load(Ordering::Acquire), 0);
        assert_eq!(stops.load(Ordering::Acquire), 0);
    }
}
