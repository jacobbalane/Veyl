//! The version-one recording folder is also the input for exact, local Link replay.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    io::{self, Read},
    path::Path,
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use veyl_link::audio::AudioBlock;
use veyl_sim::{basic::quantize, BlockMeta, Rendered};

#[derive(Clone, Serialize, Deserialize)]
pub struct RecordingMeta {
    pub format: u32,
    pub sample_rate: u32,
    pub ref_channels: u8,
    pub mic_channels: u8,
    pub source: String,
    pub device: Value,
    pub app_versions: Value,
    pub hp_cutoff_hz: u32,
    pub started_at: String,
    pub ref_first_frame: u64,
    pub mic_first_frame: u64,
    pub notes: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Label {
    pub frame: u64,
    pub kind: String,
}
pub struct Input {
    pub meta: RecordingMeta,
    pub reference: Vec<i16>,
    pub mic: Vec<i16>,
    pub blocks: Vec<BlockMeta>,
    pub labels: Vec<Label>,
    capture_origin: i64,
}
impl Input {
    pub fn generated(rendered: Rendered) -> Self {
        Self {
            meta: RecordingMeta {
                format: 1,
                sample_rate: rendered.sample_rate,
                ref_channels: rendered.ref_channels,
                mic_channels: rendered.mic_channels,
                source: "sim".into(),
                device: json!({"manufacturer":"Veyl","model":"Simulator"}),
                app_versions: json!({"lab":env!("CARGO_PKG_VERSION")}),
                hp_cutoff_hz: 8000,
                started_at: OffsetDateTime::now_utc()
                    .format(&Rfc3339)
                    .expect("UTC time"),
                ref_first_frame: 0,
                mic_first_frame: 0,
                notes: String::new(),
            },
            reference: rendered.reference.into_iter().map(quantize).collect(),
            mic: rendered.mic.into_iter().map(quantize).collect(),
            blocks: rendered.blocks,
            labels: rendered
                .labels
                .into_iter()
                .map(|poke| Label {
                    frame: poke.turn_frame,
                    kind: "poke_truth".into(),
                })
                .collect(),
            capture_origin: 0,
        }
    }
    pub fn read(directory: &Path) -> io::Result<Self> {
        let meta: RecordingMeta =
            serde_json::from_slice(&bounded(&directory.join("meta.json"), 1_048_576)?)
                .map_err(|_| invalid("Invalid recording metadata."))?;
        if meta.format != 1
            || ![44100, 48000].contains(&meta.sample_rate)
            || !(1..=2).contains(&meta.ref_channels)
            || !(1..=4).contains(&meta.mic_channels)
            || ![6000, 7000, 8000].contains(&meta.hp_cutoff_hz)
            || !["capture", "player", "test_sound", "pc", "sim"].contains(&meta.source.as_str())
        {
            return Err(invalid("Unsupported recording format."));
        }
        let reference = read_wav(
            &directory.join("ref.wav"),
            meta.sample_rate,
            meta.ref_channels,
        )?;
        let mic = read_wav(
            &directory.join("mic.wav"),
            meta.sample_rate,
            meta.mic_channels,
        )?;
        let blocks: Vec<BlockMeta> = json_lines(&directory.join("blocks.jsonl"), 32 * 1024 * 1024)?;
        if blocks.len() > 120_000 {
            return Err(invalid("Recording has too many blocks."));
        }
        let labels = match fs::metadata(directory.join("labels.jsonl")) {
            Ok(_) => json_lines(&directory.join("labels.jsonl"), 4 * 1024 * 1024)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => vec![],
            Err(error) => return Err(error),
        };
        let capture_origin = blocks
            .iter()
            .map(|block| block.capture_ns)
            .filter(|&time| time != i64::MIN)
            .min()
            .unwrap_or(0);
        let input = Self {
            meta,
            reference,
            mic,
            blocks,
            labels,
            capture_origin,
        };
        for index in 0..input.blocks.len() {
            input.block(index, 0)?;
        }
        Ok(input)
    }
    pub fn block(&self, index: usize, start_ns: i64) -> io::Result<AudioBlock> {
        let block = self
            .blocks
            .get(index)
            .ok_or_else(|| invalid("Block index is outside the recording."))?;
        let (audio, channels, first) = match block.stream {
            0 => (
                &self.reference,
                self.meta.ref_channels,
                self.meta.ref_first_frame,
            ),
            1 => (&self.mic, self.meta.mic_channels, self.meta.mic_first_frame),
            _ => return Err(invalid("Invalid recording stream.")),
        };
        let offset = block
            .first_frame
            .checked_sub(first)
            .and_then(|frame| usize::try_from(frame).ok())
            .and_then(|frame| frame.checked_mul(channels as usize))
            .ok_or_else(|| invalid("Invalid recording frame offset."))?;
        let frames = (self.meta.sample_rate / 100) as u16;
        let count = frames as usize * channels as usize;
        let end = offset
            .checked_add(count)
            .ok_or_else(|| invalid("Recording frame overflow."))?;
        let samples = audio
            .get(offset..end)
            .ok_or_else(|| invalid("Block extends beyond the WAV file."))?
            .to_vec();
        let capture_ns = if block.capture_ns == i64::MIN {
            i64::MIN
        } else {
            block
                .capture_ns
                .checked_sub(self.capture_origin)
                .and_then(|time| time.checked_add(start_ns))
                .ok_or_else(|| invalid("Recording timestamp overflow."))?
        };
        let packet = AudioBlock {
            stream: block.stream,
            channels,
            sample_rate: self.meta.sample_rate,
            seq: block.seq,
            first_frame: block.first_frame,
            capture_ns,
            frames,
            flags: block.flags,
            pre_rms_dbfs: block.pre_rms_dbfs,
            pre_peak: block.pre_peak,
            samples,
        };
        packet.encode()?;
        Ok(packet)
    }
    pub fn due_ns(&self, index: usize) -> io::Result<u64> {
        let block = self
            .blocks
            .get(index)
            .ok_or_else(|| invalid("Invalid block index."))?;
        let first = if block.stream == 0 {
            self.meta.ref_first_frame
        } else {
            self.meta.mic_first_frame
        };
        block
            .first_frame
            .checked_sub(first)
            .and_then(|frame| frame.checked_mul(1_000_000_000))
            .map(|time| time / self.meta.sample_rate as u64)
            .ok_or_else(|| invalid("Invalid frame time."))
    }
}
fn bounded(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    let file = fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(invalid("Recording metadata exceeds its size limit."));
    }
    Ok(bytes)
}
fn json_lines<T: serde::de::DeserializeOwned>(path: &Path, limit: usize) -> io::Result<Vec<T>> {
    let bytes = bounded(path, limit)?;
    let text =
        std::str::from_utf8(&bytes).map_err(|_| invalid("Recording metadata is not UTF-8."))?;
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line).map_err(|_| invalid("Invalid recording block metadata."))
        })
        .collect()
}
fn read_wav(path: &Path, rate: u32, channels: u8) -> io::Result<Vec<i16>> {
    let mut reader =
        hound::WavReader::open(path).map_err(|_| invalid("Could not open a recording WAV."))?;
    let spec = reader.spec();
    if spec.sample_format != hound::SampleFormat::Int
        || spec.bits_per_sample != 16
        || spec.sample_rate != rate
        || spec.channels != channels as u16
        || reader.duration() > rate * 600
    {
        return Err(invalid(
            "Use matching 16-bit PCM WAVs up to ten minutes long.",
        ));
    }
    reader
        .samples::<i16>()
        .map(|sample| sample.map_err(|_| invalid("Truncated recording WAV.")))
        .collect()
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_uses_absolute_frame_offsets_preserves_gaps_and_rebases_only_known_timestamps() {
        let mut input = Input::generated(veyl_sim::poke_basic(1, 0.1).unwrap());
        let directory = tempfile::tempdir().unwrap();
        input.meta.ref_first_frame = 1000;
        input.meta.mic_first_frame = 2000;
        for block in &mut input.blocks {
            block.first_frame += if block.stream == 0 { 1000 } else { 2000 };
            block.capture_ns += 7_000_000_000;
        }
        input.blocks.retain(|block| block.seq != 3);
        input
            .blocks
            .iter_mut()
            .filter(|block| block.seq == 4)
            .for_each(|block| block.flags |= 1);
        input.blocks[0].capture_ns = i64::MIN;
        fs::write(
            directory.path().join("meta.json"),
            serde_json::to_vec(&input.meta).unwrap(),
        )
        .unwrap();
        for (file, channels, data) in [("ref.wav", 2, &input.reference), ("mic.wav", 2, &input.mic)]
        {
            let spec = hound::WavSpec {
                channels,
                sample_rate: 48000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            let mut writer = hound::WavWriter::create(directory.path().join(file), spec).unwrap();
            for &sample in data {
                writer.write_sample(sample).unwrap();
            }
            writer.finalize().unwrap();
        }
        let text = input
            .blocks
            .iter()
            .map(|block| format!("{}\n", serde_json::to_string(block).unwrap()))
            .collect::<String>();
        fs::write(directory.path().join("blocks.jsonl"), text).unwrap();
        let restored = Input::read(directory.path()).unwrap();
        assert_eq!(restored.blocks.len(), 18);
        let first = restored.block(0, 123).unwrap();
        assert_eq!(first.capture_ns, i64::MIN);
        assert_eq!(first.first_frame, 1000);
        assert!(first.samples == input.reference[..960]);
        let microphone = restored.block(1, 123).unwrap();
        assert_eq!(microphone.first_frame, 2000);
        assert_eq!(microphone.capture_ns, 123);
        assert_eq!(restored.due_ns(6).unwrap(), 40_000_000);
        assert_eq!(restored.block(6, 123).unwrap().flags & 1, 1);
    }
}
