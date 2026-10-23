//! Shared capture conversion, privacy filtering and block cutting; raw PCM stays on its owner.
use audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Async, FixedAsync, Resampler, SincInterpolationParameters};
use serde::Serialize;
use std::{collections::VecDeque, io};
use veyl_dsp::filters::PrivacyFilter;
use veyl_link::audio::AudioBlock;

#[cfg(windows)]
pub(crate) mod native;
mod workers;
pub use workers::CaptureGroup;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureDetails {
    pub format: CaptureFormat,
    pub raw_mode: bool,
    pub event_driven: bool,
    pub buffer_ms: f64,
    pub pro_audio_thread: bool,
}
/// A backend owns its handles on the capture thread and lends packets only during poll.
pub trait CaptureBackend {
    fn details(&self) -> CaptureDetails;
    fn start(&mut self) -> io::Result<()>;
    fn wait(&mut self) -> io::Result<()>;
    fn poll(&mut self, sink: &mut dyn FnMut(CapturePacket<'_>) -> io::Result<()>)
        -> io::Result<()>;
    fn stop(&mut self);
}
pub type CaptureFactory = std::sync::Arc<
    dyn Fn(&crate::pc_devices::AudioDevice, u8) -> io::Result<Box<dyn CaptureBackend>>
        + Send
        + Sync,
>;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Encoding {
    Signed16,
    Signed24,
    Signed32,
    Float32,
}
impl Encoding {
    fn bytes(self) -> usize {
        match self {
            Self::Signed16 => 2,
            Self::Signed24 => 3,
            _ => 4,
        }
    }
    fn sample(self, bytes: &[u8]) -> f32 {
        match self {
            Self::Signed16 => i16::from_le_bytes(bytes[..2].try_into().unwrap()) as f32 / 32768.0,
            Self::Signed24 => {
                let value = i32::from_le_bytes([0, bytes[0], bytes[1], bytes[2]]);
                value as f32 / 2147483648.0
            }
            Self::Signed32 => {
                i32::from_le_bytes(bytes[..4].try_into().unwrap()) as f32 / 2147483648.0
            }
            Self::Float32 => {
                let value = f32::from_le_bytes(bytes[..4].try_into().unwrap());
                if value.is_finite() {
                    value.clamp(-8.0, 8.0)
                } else {
                    0.0
                }
            }
        }
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub encoding: Encoding,
    pub frame_stride: usize,
}
impl CaptureFormat {
    pub fn validate(&self) -> io::Result<()> {
        if !(8000..=384000).contains(&self.sample_rate)
            || !(1..=64).contains(&self.channels)
            || self.frame_stride < self.channels as usize * self.encoding.bytes()
            || self.frame_stride > 65535
        {
            return Err(invalid("Unsupported Windows audio format."));
        }
        Ok(())
    }
    pub fn output_channels(&self) -> u8 {
        self.channels.min(2) as u8
    }
}
/// The native backend lends its packet only until ReleaseBuffer; no raw buffer crosses threads.
pub struct CapturePacket<'a> {
    pub bytes: &'a [u8],
    pub frames: usize,
    pub device_position: Option<u64>,
    pub capture_ns: Option<i64>,
    pub discontinuity: bool,
    pub silent: bool,
}

pub struct Processor {
    format: CaptureFormat,
    stream: u8,
    channels: usize,
    input_chunk: usize,
    raw: VecDeque<f32>,
    input_buffer: Vec<f32>,
    output_buffer: Vec<f32>,
    resampler: Option<Async<f32>>,
    filter: PrivacyFilter,
    pending: Vec<f32>,
    pending_ns: Option<i64>,
    cursor: u64,
    segment_input: u64,
    segment_output: u64,
    segment_ns: Option<i64>,
    expected_device_frame: Option<u64>,
    sequence: u32,
    gap: bool,
}
impl Processor {
    pub fn new(format: CaptureFormat, stream: u8) -> io::Result<Self> {
        format.validate()?;
        if stream > 1 {
            return Err(invalid("Invalid PC audio stream."));
        }
        let channels = format.output_channels() as usize;
        let input_chunk = (format.sample_rate / 50) as usize;
        let resampler = if format.sample_rate == 48000 {
            None
        } else {
            Some(
                Async::<f32>::new_sinc(
                    48000.0 / format.sample_rate as f64,
                    1.0,
                    &SincInterpolationParameters::default(),
                    input_chunk,
                    channels,
                    FixedAsync::Input,
                )
                .map_err(|_| invalid("Could not create the PC resampler."))?,
            )
        };
        let maximum_output = resampler
            .as_ref()
            .map(Resampler::output_frames_max)
            .unwrap_or(input_chunk);
        Ok(Self {
            format,
            stream,
            channels,
            input_chunk,
            raw: VecDeque::with_capacity(input_chunk * channels * 2),
            input_buffer: vec![0.0; input_chunk * channels],
            output_buffer: vec![0.0; maximum_output * channels],
            resampler,
            filter: PrivacyFilter::new(48000, 8000, channels).map_err(io::Error::other)?,
            pending: Vec::with_capacity(480 * channels),
            pending_ns: None,
            cursor: 0,
            segment_input: 0,
            segment_output: 0,
            segment_ns: None,
            expected_device_frame: None,
            sequence: 0,
            gap: false,
        })
    }
    pub fn output_channels(&self) -> u8 {
        self.channels as u8
    }
    pub fn resampling(&self) -> bool {
        self.resampler.is_some()
    }
    pub fn delay_frames(&self) -> usize {
        self.resampler
            .as_ref()
            .map(Resampler::output_delay)
            .unwrap_or(0)
    }
    pub fn push(
        &mut self,
        packet: CapturePacket<'_>,
        sink: &mut dyn FnMut(AudioBlock) -> io::Result<()>,
    ) -> io::Result<()> {
        if packet.frames > self.format.sample_rate as usize
            || !packet.silent
                && packet.bytes.len()
                    < packet
                        .frames
                        .checked_mul(self.format.frame_stride)
                        .ok_or_else(|| invalid("Capture size overflow."))?
        {
            return Err(invalid("Invalid Windows capture packet."));
        }
        if packet.frames == 0 {
            return Ok(());
        }
        let missing = self
            .expected_device_frame
            .zip(packet.device_position)
            .and_then(|(expected, actual)| actual.checked_sub(expected))
            .unwrap_or(0);
        let moved_back = self
            .expected_device_frame
            .zip(packet.device_position)
            .is_some_and(|(expected, actual)| actual < expected);
        if packet.discontinuity || missing != 0 || moved_back {
            let lost_input = missing as u128 + (self.raw.len() / self.channels) as u128;
            let lost_output = ((lost_input * 48000 + self.format.sample_rate as u128 / 2)
                / self.format.sample_rate as u128)
                .min(u64::MAX as u128) as u64;
            self.cursor = self
                .cursor
                .checked_add(lost_output)
                .ok_or_else(|| invalid("PC frame overflow."))?;
            self.raw.clear();
            self.pending.clear();
            self.pending_ns = None;
            self.filter.reset();
            if let Some(resampler) = &mut self.resampler {
                resampler.reset();
            }
            self.segment_input = 0;
            self.segment_output = 0;
            self.segment_ns = None;
            self.gap = true;
        }
        self.expected_device_frame = packet
            .device_position
            .and_then(|first| first.checked_add(packet.frames as u64));
        self.segment_ns = packet
            .capture_ns
            .and_then(|time| shifted(time, -(self.segment_input as i128), self.format.sample_rate));
        self.segment_input = self
            .segment_input
            .checked_add(packet.frames as u64)
            .ok_or_else(|| invalid("PC frame overflow."))?;
        let bytes_per_sample = self.format.encoding.bytes();
        for frame in 0..packet.frames {
            let mut values = [0.0; 2];
            let mut counts = [0_u16; 2];
            for channel in 0..self.format.channels as usize {
                let value = if packet.silent {
                    0.0
                } else {
                    let offset = frame * self.format.frame_stride + channel * bytes_per_sample;
                    self.format
                        .encoding
                        .sample(&packet.bytes[offset..offset + bytes_per_sample])
                };
                let output = channel % self.channels;
                values[output] += value;
                counts[output] += 1;
            }
            for channel in 0..self.channels {
                self.raw.push_back(values[channel] / counts[channel] as f32);
            }
            // Keep the raw queue bounded even when a driver returns a large packet.
            if self.raw.len() == self.input_chunk * self.channels {
                self.process_chunk(sink)?;
            }
        }
        Ok(())
    }
    fn process_chunk(
        &mut self,
        sink: &mut dyn FnMut(AudioBlock) -> io::Result<()>,
    ) -> io::Result<()> {
        for sample in &mut self.input_buffer {
            *sample = self.raw.pop_front().unwrap();
        }
        let output_frames = if let Some(resampler) = &mut self.resampler {
            let input = InterleavedSlice::new(&self.input_buffer, self.channels, self.input_chunk)
                .map_err(io::Error::other)?;
            let output_frames = resampler.output_frames_max();
            let mut output =
                InterleavedSlice::new_mut(&mut self.output_buffer, self.channels, output_frames)
                    .map_err(io::Error::other)?;
            let (used, produced) = resampler
                .process_into_buffer(&input, &mut output, None)
                .map_err(|_| invalid("PC resampling failed."))?;
            if used != self.input_chunk {
                return Err(invalid("PC resampler did not consume its input."));
            }
            produced
        } else {
            self.output_buffer[..self.input_buffer.len()].copy_from_slice(&self.input_buffer);
            self.input_chunk
        };
        let delay = self.delay_frames() as i128;
        for frame in 0..output_frames {
            if self.pending.is_empty() {
                self.pending_ns = self
                    .segment_ns
                    .and_then(|time| shifted(time, self.segment_output as i128 - delay, 48000));
            }
            self.pending.extend_from_slice(
                &self.output_buffer[frame * self.channels..(frame + 1) * self.channels],
            );
            self.cursor = self
                .cursor
                .checked_add(1)
                .ok_or_else(|| invalid("PC frame overflow."))?;
            self.segment_output += 1;
            if self.pending.len() == 480 * self.channels {
                let power = self
                    .pending
                    .iter()
                    .map(|&sample| (sample as f64).powi(2))
                    .sum::<f64>()
                    / self.pending.len() as f64;
                let pre_rms_dbfs = (10.0 * power.max(1e-12).log10()) as f32;
                let pre_peak = self
                    .pending
                    .iter()
                    .map(|sample| sample.abs())
                    .fold(0.0, f32::max);
                self.filter
                    .process_interleaved(&mut self.pending)
                    .map_err(io::Error::other)?;
                let samples = self
                    .pending
                    .iter()
                    .map(|&value| (value * 32768.0).round().clamp(-32768.0, 32767.0) as i16)
                    .collect();
                let flags = 4
                    | if self.stream == 0 { 8 } else { 0 }
                    | if self.gap { 1 } else { 0 }
                    | if pre_peak >= 0.99 { 2 } else { 0 };
                sink(AudioBlock {
                    stream: self.stream,
                    channels: self.channels as u8,
                    sample_rate: 48000,
                    seq: self.sequence,
                    first_frame: self.cursor - 480,
                    capture_ns: self.pending_ns.unwrap_or(i64::MIN),
                    frames: 480,
                    flags,
                    pre_rms_dbfs,
                    pre_peak,
                    samples,
                })?;
                self.sequence = self.sequence.wrapping_add(1);
                self.gap = false;
                self.pending.clear();
            }
        }
        Ok(())
    }
}
fn shifted(time: i64, frames: i128, rate: u32) -> Option<i64> {
    i64::try_from(time as i128 + frames * 1_000_000_000 / rate as i128).ok()
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn format(rate: u32, channels: u16, encoding: Encoding) -> CaptureFormat {
        CaptureFormat {
            sample_rate: rate,
            channels,
            encoding,
            frame_stride: channels as usize * encoding.bytes(),
        }
    }
    fn bytes(samples: &[f32]) -> Vec<u8> {
        samples
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }
    fn packet(
        data: &[u8],
        frames: usize,
        position: u64,
        time: Option<i64>,
        gap: bool,
    ) -> CapturePacket<'_> {
        CapturePacket {
            bytes: data,
            frames,
            device_position: Some(position),
            capture_ns: time,
            discontinuity: gap,
            silent: false,
        }
    }
    #[test]
    fn signed_pcm_float_and_padding_conversion_are_correct_and_nonfinite_values_are_silent() {
        assert_eq!(Encoding::Signed16.sample(&i16::MIN.to_le_bytes()), -1.0);
        assert_eq!(Encoding::Signed24.sample(&[0, 0, 128]), -1.0);
        assert_eq!(Encoding::Signed24.sample(&[0, 0, 64]), 0.5);
        assert_eq!(Encoding::Signed32.sample(&i32::MIN.to_le_bytes()), -1.0);
        assert_eq!(
            Encoding::Signed32.sample(&1073741824_i32.to_le_bytes()),
            0.5
        );
        assert_eq!(Encoding::Float32.sample(&f32::NAN.to_le_bytes()), 0.0);
        assert_eq!(Encoding::Float32.sample(&f32::INFINITY.to_le_bytes()), 0.0);
        let mut padded = format(48000, 1, Encoding::Signed24);
        padded.frame_stride = 4;
        let data = [0, 0, 64, 123].repeat(960);
        let mut blocks = vec![];
        Processor::new(padded, 1)
            .unwrap()
            .push(packet(&data, 960, 0, Some(123), false), &mut |block| {
                blocks.push(block);
                Ok(())
            })
            .unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].pre_peak, 0.5);
        assert!((blocks[0].pre_rms_dbfs + 6.0206).abs() < 0.001);
    }
    #[test]
    fn stereo_downmix_levels_flags_and_timestamps_are_cut_in_exact_ten_ms_blocks() {
        let data = bytes(&[1.0, 0.0, 1.0, 0.0, 1.0, 0.0].repeat(960));
        let mut processor = Processor::new(format(48000, 6, Encoding::Float32), 0).unwrap();
        let mut blocks = vec![];
        processor
            .push(
                packet(&data, 960, 500, Some(1_000_000_000), false),
                &mut |block| {
                    blocks.push(block);
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!((blocks[0].first_frame, blocks[1].first_frame), (0, 480));
        assert_eq!(
            (blocks[0].capture_ns, blocks[1].capture_ns),
            (1_000_000_000, 1_010_000_000)
        );
        assert_eq!(blocks[0].flags, 14);
        assert_eq!(blocks[0].channels, 2);
        assert_eq!(blocks[0].pre_peak, 1.0);
        assert!(blocks
            .iter()
            .flat_map(|block| block.samples.iter().skip(1).step_by(2))
            .all(|&value| value == 0));
    }
    #[test]
    fn device_gaps_drop_partial_blocks_and_filter_tails_without_reusing_frame_numbers() {
        let mut processor = Processor::new(format(48000, 1, Encoding::Float32), 1).unwrap();
        let mut blocks = vec![];
        let mut sink = |block| {
            blocks.push(block);
            Ok(())
        };
        processor
            .push(
                packet(&bytes(&vec![0.5; 1200]), 1200, 0, Some(0), false),
                &mut sink,
            )
            .unwrap();
        processor
            .push(packet(&[], 960, 1680, None, true).with_silence(), &mut sink)
            .unwrap();
        assert_eq!(blocks.len(), 4);
        assert_eq!(blocks[2].first_frame, 1680);
        assert_eq!(blocks[2].flags & 1, 1);
        assert_eq!(blocks[3].flags & 1, 0);
        assert_eq!(blocks[2].capture_ns, i64::MIN);
        assert!(blocks[2].samples.iter().all(|&value| value == 0));
        assert!(processor
            .push(packet(&[0; 1], 480, 0, None, false), &mut |_| Ok(()))
            .is_err());
    }
    #[test]
    fn sinc_resampling_keeps_pitch_and_high_sounds_while_the_privacy_filter_removes_voice() {
        for rate in [32000, 44100, 48000, 96000] {
            for frequency in [1000.0_f32, 12000.0] {
                let mut processor = Processor::new(format(rate, 1, Encoding::Float32), 1).unwrap();
                let mut blocks = vec![];
                for chunk in 0..100 {
                    let first = chunk * (rate / 100);
                    let data = bytes(
                        &(0..rate / 100)
                            .map(|index| {
                                (std::f32::consts::TAU * frequency * (first + index) as f32
                                    / rate as f32)
                                    .sin()
                                    * 0.2
                            })
                            .collect::<Vec<_>>(),
                    );
                    processor
                        .push(
                            packet(
                                &data,
                                (rate / 100) as usize,
                                first as u64,
                                Some(chunk as i64 * 10_000_000),
                                false,
                            ),
                            &mut |block| {
                                blocks.push(block);
                                Ok(())
                            },
                        )
                        .unwrap();
                }
                assert!((98..=100).contains(&blocks.len()));
                let signal: Vec<_> = blocks
                    .iter()
                    .skip(10)
                    .flat_map(|block| block.samples.iter().map(|&sample| sample as f64 / 32768.0))
                    .collect();
                let rms = (signal.iter().map(|value| value * value).sum::<f64>()
                    / signal.len() as f64)
                    .sqrt();
                if frequency < 8000.0 {
                    assert!(rms < 0.0001);
                } else {
                    assert!((0.12..0.16).contains(&rms));
                    let crossings = signal
                        .windows(2)
                        .filter(|pair| pair[0] <= 0.0 && pair[1] > 0.0)
                        .count();
                    let measured = crossings as f64 * 48000.0 / signal.len() as f64;
                    assert!((measured - frequency as f64).abs() < 3.0);
                }
            }
        }
    }
    impl CapturePacket<'_> {
        fn with_silence(mut self) -> Self {
            self.silent = true;
            self
        }
    }
}
