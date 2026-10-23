//! Frame-timed sound quality and source safety, without capture, clocks or I/O.
use crate::{echo::EchoFrame, BlockFlags, BlockIn, Stream};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundSource {
    #[default]
    Capture,
    Player,
    TestSound,
    Pc,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundRoute {
    #[default]
    Speaker,
    External,
    Headphones,
    Bluetooth,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceStatus {
    pub capture_running: bool,
    pub source: SoundSource,
    pub route: SoundRoute,
    pub mic_silenced: bool,
    pub mic_available: bool,
    pub media_volume: Option<f32>,
}
impl Default for SourceStatus {
    fn default() -> Self {
        Self {
            capture_running: false,
            source: SoundSource::Capture,
            route: SoundRoute::Speaker,
            mic_silenced: false,
            mic_available: true,
            media_volume: None,
        }
    }
}
impl SourceStatus {
    pub fn permits_detection(self, pc: bool) -> bool {
        self.capture_running
            && self.mic_available
            && !self.mic_silenced
            && if pc {
                self.route != SoundRoute::Headphones && self.route != SoundRoute::External
            } else {
                self.route == SoundRoute::Speaker
            }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeterLabel {
    #[default]
    None,
    Poor,
    Fair,
    Good,
    Great,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tip {
    #[default]
    CaptureOff,
    NotSpeaker,
    PcHeadphones,
    PcNoMic,
    CaptureBlocked,
    NothingPlaying,
    MicSilenced,
    Clipping,
    VolumeLow,
    FewHighSounds,
    Gaps,
    OtherSounds,
    UnclearEcho,
    NoisyRoom,
    PcBluetooth,
    Ready,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct MeterFactors {
    pub high: f32,
    pub detail: f32,
    pub volume: f32,
    pub steady: f32,
    pub clarity: f32,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MeterReading {
    pub frame: u64,
    pub score: u8,
    pub label: MeterLabel,
    pub tip: Tip,
    pub factors: MeterFactors,
    pub brightness_db: Option<f32>,
    pub bandwidth_hz: f32,
    pub mic_band_dbfs: f32,
    pub clarity_db: f32,
    pub steady_fraction: f32,
    pub mic_clip: f32,
    pub detail_cm: Option<f32>,
}
impl Default for MeterReading {
    fn default() -> Self {
        Self {
            frame: 0,
            score: 0,
            label: MeterLabel::None,
            tip: Tip::CaptureOff,
            factors: MeterFactors::default(),
            brightness_db: None,
            bandwidth_hz: 0.0,
            mic_band_dbfs: -120.0,
            clarity_db: -120.0,
            steady_fraction: 0.0,
            mic_clip: 0.0,
            detail_cm: None,
        }
    }
}
#[derive(Clone, Copy, Default)]
struct BlockLevel {
    power: f32,
    pre_dbfs: f32,
    clipped: bool,
    valid: bool,
}
#[derive(Clone, Copy)]
struct EchoLevel {
    frame: u64,
    clarity: f32,
    bandwidth: f32,
}
pub(crate) struct Meter {
    rate: u32,
    pc: bool,
    speed: f32,
    status: SourceStatus,
    reference: VecDeque<BlockLevel>,
    microphone: VecDeque<BlockLevel>,
    echo: VecDeque<EchoLevel>,
    reference_end: Option<u64>,
    microphone_end: Option<u64>,
    ref_window_count: usize,
    ref_window_power: f32,
    steady: VecDeque<bool>,
    zero_frames: u64,
    quiet_frames: u64,
    last_tick: u64,
    last_tip: u64,
    pending: bool,
    pending_tip: bool,
    reading: MeterReading,
}
fn level(power: f32) -> f32 {
    10.0 * power.max(1e-12).log10()
}
fn curve(value: f32, points: &[(f32, f32)]) -> f32 {
    if value <= points[0].0 {
        return points[0].1;
    }
    for pair in points.windows(2) {
        if value <= pair[1].0 {
            return pair[0].1
                + (pair[1].1 - pair[0].1) * (value - pair[0].0) / (pair[1].0 - pair[0].0);
        }
    }
    points.last().unwrap().1
}
fn push_bounded<T>(ring: &mut VecDeque<T>, value: T, limit: usize) {
    if ring.len() == limit {
        ring.pop_front();
    }
    ring.push_back(value);
}
impl Meter {
    pub fn new(rate: u32, hop: usize, pc: bool, temperature_c: f32) -> Self {
        Self {
            rate,
            pc,
            speed: 331.3 + 0.606 * temperature_c,
            status: SourceStatus::default(),
            reference: VecDeque::with_capacity(10),
            microphone: VecDeque::with_capacity(100),
            echo: VecDeque::with_capacity((rate as usize / 10).div_ceil(hop) + 2),
            reference_end: None,
            microphone_end: None,
            ref_window_count: 0,
            ref_window_power: 0.0,
            steady: VecDeque::with_capacity(30),
            zero_frames: 0,
            quiet_frames: 0,
            last_tick: 0,
            last_tip: 0,
            pending: false,
            pending_tip: false,
            reading: MeterReading::default(),
        }
    }
    pub fn reading(&self) -> MeterReading {
        self.reading
    }
    pub fn status(&self) -> SourceStatus {
        self.status
    }
    pub fn set_status(&mut self, status: SourceStatus) -> Result<(), &'static str> {
        if status
            .media_volume
            .is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
        {
            return Err("Invalid source volume.");
        }
        if self.status != status {
            self.pending_tip |=
                !self.status.permits_detection(self.pc) || !status.permits_detection(self.pc);
            self.status = status;
            self.pending = true;
        }
        if !status.permits_detection(self.pc) {
            self.reading.score = 0;
            self.reading.label = MeterLabel::None;
        }
        Ok(())
    }
    pub fn reference_active(&self) -> bool {
        !self.reference.is_empty()
            && self
                .reference
                .back()
                .is_some_and(|value| level(value.power) >= -70.0)
            && level(
                self.reference.iter().map(|v| v.power).sum::<f32>() / self.reference.len() as f32,
            ) >= -70.0
    }
    fn reference_level(&mut self, value: BlockLevel) {
        push_bounded(&mut self.reference, value, 10);
        self.ref_window_count += 1;
        self.ref_window_power += value.power;
        if self.ref_window_count == 10 {
            push_bounded(
                &mut self.steady,
                level(self.ref_window_power / 10.0) >= -70.0,
                30,
            );
            self.ref_window_count = 0;
            self.ref_window_power = 0.0;
        }
    }
    /// Only accepted blocks enter the meter. Missing frames become silence in its rolling windows.
    pub fn observe_block(&mut self, block: &BlockIn<'_>, channels: usize) {
        let frames = self.rate as u64 / 100;
        let previous = if block.stream == Stream::Reference {
            self.reference_end
        } else {
            self.microphone_end
        };
        let missing = previous
            .map_or(0, |end| block.first_frame.saturating_sub(end) / frames)
            .min(300);
        let gap = missing > 0 || block.flags.0 & BlockFlags::GAP_BEFORE != 0;
        if gap && block.stream == Stream::Reference {
            self.zero_frames = 0;
            self.quiet_frames = 0;
        }
        for _ in 0..missing {
            let absent = BlockLevel {
                pre_dbfs: -120.0,
                ..BlockLevel::default()
            };
            if block.stream == Stream::Reference {
                self.reference_level(absent);
            } else {
                push_bounded(&mut self.microphone, absent, 100);
            }
        }
        let power = block
            .samples
            .iter()
            .map(|sample| (*sample as f32 / 32768.0).powi(2))
            .sum::<f32>()
            / block.samples.len().max(1) as f32;
        let value = BlockLevel {
            power,
            pre_dbfs: block.pre_rms_dbfs,
            clipped: block.flags.0 & BlockFlags::CLIPPED != 0,
            valid: true,
        };
        let end = block
            .first_frame
            .saturating_add((block.samples.len() / channels) as u64);
        if block.stream == Stream::Reference {
            self.reference_level(value);
            self.reference_end = Some(end);
            self.zero_frames = if block.pre_peak == 0.0 {
                self.zero_frames.saturating_add(frames)
            } else {
                0
            };
            self.quiet_frames = if block.pre_rms_dbfs <= -90.0 {
                self.quiet_frames.saturating_add(frames)
            } else {
                0
            };
        } else {
            if self.microphone_end.is_none() {
                self.last_tick = block.first_frame;
                self.last_tip = block.first_frame;
            }
            push_bounded(&mut self.microphone, value, 100);
            self.microphone_end = Some(end);
        }
    }
    pub fn observe_echo(&mut self, frame: u64, echo: EchoFrame) {
        let clarity = (10.0
            * (echo.predicted_power.max(1e-20) / echo.residual_power.max(1e-20)).log10())
        .clamp(-120.0, 120.0) as f32;
        let limit = self.echo.capacity();
        push_bounded(
            &mut self.echo,
            EchoLevel {
                frame,
                clarity,
                bandwidth: echo.bandwidth_hz,
            },
            limit,
        );
    }
    fn tip(&self, factors: MeterFactors, microphone: f32, mic_clip: f32, noisy: bool) -> Tip {
        let status = self.status;
        if !status.capture_running {
            Tip::CaptureOff
        } else if !self.pc && status.route != SoundRoute::Speaker {
            Tip::NotSpeaker
        } else if self.pc && status.route == SoundRoute::Headphones {
            Tip::PcHeadphones
        } else if self.pc && !status.mic_available {
            Tip::PcNoMic
        } else if matches!(status.source, SoundSource::Capture | SoundSource::Pc)
            && self.zero_frames >= 2 * self.rate as u64
            && microphone >= -60.0
        {
            Tip::CaptureBlocked
        } else if self.quiet_frames >= 2 * self.rate as u64 && microphone < -60.0 {
            Tip::NothingPlaying
        } else if status.mic_silenced {
            Tip::MicSilenced
        } else if mic_clip > 0.001 {
            Tip::Clipping
        } else if factors.volume < 0.4 {
            Tip::VolumeLow
        } else if factors.high < 0.3 || factors.detail < 0.3 {
            Tip::FewHighSounds
        } else if factors.steady < 0.5 {
            Tip::Gaps
        } else if status.source == SoundSource::TestSound && factors.clarity < 0.5 {
            Tip::OtherSounds
        } else if factors.clarity < 0.4 {
            Tip::UnclearEcho
        } else if noisy {
            Tip::NoisyRoom
        } else if self.pc && status.route == SoundRoute::Bluetooth {
            Tip::PcBluetooth
        } else {
            Tip::Ready
        }
    }
    pub fn tick(&mut self, frame: u64, has_audio: bool, noisy: bool) -> Option<MeterReading> {
        let step = self.rate as u64 / 10;
        if !self.pending && frame.saturating_sub(self.last_tick) < step {
            return None;
        }
        let elapsed = frame.saturating_sub(self.last_tick);
        self.last_tick = frame;
        self.pending = false;
        let urgent_tip = std::mem::take(&mut self.pending_tip);
        let oldest = frame.saturating_sub(step);
        while self.echo.front().is_some_and(|v| v.frame < oldest) {
            self.echo.pop_front();
        }
        let ref_count = self.reference.len().max(1) as f32;
        let ref_power = self.reference.iter().map(|v| v.power).sum::<f32>() / ref_count;
        let valid_refs = self.reference.iter().filter(|v| v.valid).count().max(1) as f32;
        let pre = self
            .reference
            .iter()
            .filter(|v| v.valid)
            .map(|v| v.pre_dbfs)
            .sum::<f32>()
            / valid_refs;
        let brightness =
            (!self.reference.is_empty() && pre > -90.0).then(|| level(ref_power) - pre);
        let mic_count = self.microphone.len().clamp(1, 10) as f32;
        let mic_level = level(
            self.microphone
                .iter()
                .rev()
                .take(10)
                .map(|v| v.power)
                .sum::<f32>()
                / mic_count,
        );
        let clipped = self.microphone.iter().filter(|v| v.clipped).count() as f32
            / self.microphone.iter().filter(|v| v.valid).count().max(1) as f32;
        let echo_count = self.echo.len().max(1) as f32;
        let bandwidth = self.echo.iter().map(|v| v.bandwidth).sum::<f32>() / echo_count;
        let clarity = if self.echo.is_empty() {
            -120.0
        } else {
            self.echo.iter().map(|v| v.clarity).sum::<f32>() / echo_count
        };
        let steady =
            self.steady.iter().filter(|v| **v).count() as f32 / self.steady.len().max(1) as f32;
        let raw = MeterFactors {
            high: curve(
                brightness.unwrap_or(-120.0),
                &[(-45.0, 0.0), (-35.0, 0.3), (-25.0, 0.7), (-18.0, 1.0)],
            ),
            detail: curve(
                bandwidth / 1000.0,
                &[(0.0, 0.0), (2.0, 0.2), (5.0, 0.6), (9.0, 1.0)],
            ),
            volume: curve(
                mic_level,
                &[(-75.0, 0.0), (-60.0, 0.4), (-45.0, 0.85), (-38.0, 1.0)],
            )
            .min(if clipped > 0.001 { 0.6 } else { 1.0 }),
            steady: curve(steady, &[(0.5, 0.0), (1.0, 1.0)]),
            clarity: curve(
                clarity,
                &[(0.0, 0.0), (6.0, 0.4), (12.0, 0.75), (20.0, 1.0)],
            ),
        };
        let alpha = 1.0 - (-(elapsed as f32) / (0.5 * self.rate as f32)).exp();
        let mut factors = self.reading.factors;
        for (value, target) in [
            (&mut factors.high, raw.high),
            (&mut factors.detail, raw.detail),
            (&mut factors.volume, raw.volume),
            (&mut factors.steady, raw.steady),
            (&mut factors.clarity, raw.clarity),
        ] {
            *value += alpha * (target - *value);
        }
        let requested = self.tip(factors, mic_level, clipped, noisy);
        let tip = if requested != self.reading.tip
            && (urgent_tip || frame.saturating_sub(self.last_tip) >= self.rate as u64)
        {
            self.last_tip = frame;
            requested
        } else {
            self.reading.tip
        };
        if clipped > 0.001 {
            factors.volume = factors.volume.min(0.6);
        }
        let disabled = !has_audio
            || !self.status.permits_detection(self.pc)
            || matches!(requested, Tip::CaptureBlocked | Tip::NothingPlaying);
        let score = if disabled {
            0
        } else {
            (100.0
                * (0.2 * factors.high
                    + 0.2 * factors.detail
                    + 0.2 * factors.volume
                    + 0.15 * factors.steady
                    + 0.25 * factors.clarity))
                .round()
                .clamp(0.0, 100.0) as u8
        };
        let label = if disabled {
            MeterLabel::None
        } else if score >= 75 {
            MeterLabel::Great
        } else if score >= 55 {
            MeterLabel::Good
        } else if score >= 35 {
            MeterLabel::Fair
        } else {
            MeterLabel::Poor
        };
        self.reading = MeterReading {
            frame,
            score,
            label,
            tip,
            factors,
            brightness_db: brightness,
            bandwidth_hz: bandwidth,
            mic_band_dbfs: mic_level,
            clarity_db: clarity,
            steady_fraction: steady,
            mic_clip: clipped,
            detail_cm: (bandwidth >= 1000.0)
                .then(|| (1000.0 * self.speed / (2.0 * bandwidth)).round() / 10.0),
        };
        Some(self.reading)
    }
}

#[cfg(test)]
mod tests;
