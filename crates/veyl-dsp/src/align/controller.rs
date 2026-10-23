use super::{AlignmentLock, DriftTracker, GccPhat, LockChange};
use crate::{config::EngineConfig, echo::DirectPeak, ring::SampleRing, StreamFormat};

/// Owns the lag search and fast-map feedback shared by offline and live processing.
pub struct Aligner {
    config: EngineConfig,
    rate: u32,
    pc: bool,
    gcc: GccPhat,
    lock: AlignmentLock,
    drift: DriftTracker,
    reference: Vec<f32>,
    microphone: Vec<f32>,
    last_attempt: Option<u64>,
    hop_frame: Option<u64>,
    feedback_origin: Option<u64>,
    measurement_frame: f64,
    lag: i64,
    applied: f64,
    direct_reference: Option<f64>,
    residual: Option<f64>,
}
impl Aligner {
    pub fn new(config: EngineConfig, format: StreamFormat, pc: bool) -> Result<Self, &'static str> {
        format.validate()?;
        let rate = format.sample_rate;
        let gcc = GccPhat::new(rate, &config, rate as usize * 2)?;
        let frames = gcc.reference_frames();
        Ok(Self {
            lock: AlignmentLock::new(rate, config.clone())?,
            drift: DriftTracker::new(&config),
            config,
            rate,
            pc,
            reference: vec![0.0; frames],
            microphone: vec![0.0; frames + rate as usize * 2],
            gcc,
            last_attempt: None,
            hop_frame: None,
            feedback_origin: None,
            measurement_frame: 0.0,
            lag: 0,
            applied: 0.0,
            direct_reference: None,
            residual: None,
        })
    }
    pub fn locked(&self) -> bool {
        self.lock.locked()
    }
    pub fn lag(&self) -> i64 {
        self.lag
    }
    pub fn fraction(&self) -> f64 {
        self.drift.fraction()
    }
    pub fn drift_ppm(&self) -> f64 {
        self.drift.ppm()
    }
    pub fn drift_warning(&self) -> bool {
        self.drift.ppm().abs() > self.config.drift_warning_ppm
    }
    pub fn direct_residual(&self) -> Option<f64> {
        self.residual
    }
    pub fn reset(&mut self) {
        self.lock.reset();
        self.drift = DriftTracker::new(&self.config);
        self.last_attempt = None;
        self.hop_frame = None;
        self.feedback_origin = None;
        self.measurement_frame = 0.0;
        self.lag = 0;
        self.applied = 0.0;
        self.direct_reference = None;
        self.residual = None;
    }
    /// Bound missing alignment evidence separately from the stricter gesture-window validity.
    pub fn try_align(
        &mut self,
        reference: &SampleRing,
        microphone: &SampleRing,
    ) -> Option<LockChange> {
        let latest = microphone.end()?;
        let interval = (self.rate as f32
            * if self.locked() {
                self.config.recheck_seconds
            } else {
                self.config.align_interval_seconds
            }) as u64;
        if self
            .last_attempt
            .is_some_and(|last| latest.saturating_sub(last) < interval)
        {
            return None;
        }
        let frames = self.reference.len() as u64;
        let mut reference_end = reference.end()?;
        let middle = reference_end.checked_sub(frames / 2)?;
        let offset = |at| {
            reference
                .capture_time(at)
                .and_then(|ns| microphone.frame_at_time(ns))
                .map(|mic| mic - at as f64)
        };
        let timestamped = offset(middle);
        let centre = timestamped.unwrap_or(microphone.first()? as f64 - reference.first()? as f64);
        if !centre.is_finite() || centre.abs() > i64::MAX as f64 / 2.0 {
            return None;
        }
        let (minimum, maximum) = if timestamped.is_some() {
            (
                centre.floor() as i64 - (self.rate * 3 / 100) as i64,
                centre.floor() as i64
                    + if self.pc {
                        self.rate / 2
                    } else {
                        self.rate * 3 / 10
                    } as i64,
            )
        } else {
            (
                centre.floor() as i64 - self.rate as i64,
                centre.floor() as i64 + self.rate as i64,
            )
        };
        let available = u64::try_from(latest as i128 - maximum as i128).ok()?;
        reference_end = reference_end.min(available);
        let first = reference_end.checked_sub(frames)?;
        let mic_first = u64::try_from(first as i128 + minimum as i128).ok()?;
        let length = frames as usize + (maximum - minimum) as usize;
        let reference_coverage = reference.copy_with_coverage(first, None, &mut self.reference)?;
        let microphone_coverage =
            microphone.copy_with_coverage(mic_first, None, &mut self.microphone[..length])?;
        self.last_attempt = Some(latest);
        for (coverage, length) in [
            (reference_coverage, self.reference.len()),
            (microphone_coverage, length),
        ] {
            if coverage.missing_frames as f32
                > self.config.align_max_missing_fraction * length as f32
                || coverage.longest_gap as f32
                    > self.config.align_max_gap_ms * self.rate as f32 / 1000.0
            {
                self.lock.reject();
                return None;
            }
        }
        let middle = first + frames / 2;
        let timestamp_offset = offset(middle).unwrap_or(centre);
        let tracked = self.hop_frame.map(|frame| {
            let lag = self.lag as f64 + self.drift.fraction();
            // GCC measures the window centre, while fast feedback starts at the later lock hop.
            let origin_offset =
                self.feedback_origin.unwrap_or(frame) as f64 - self.measurement_frame;
            lag + self.drift.ppm() * 1e-6 * (middle as f64 + lag - frame as f64 + origin_offset)
        });
        let expected = self
            .lock
            .expected_lag(timestamp_offset, tracked)
            .map(|lag| lag - minimum as f64);
        let Some(mut estimate) = self.gcc.estimate_near(
            &self.reference,
            &self.microphone[..length],
            0..=maximum - minimum,
            expected,
        ) else {
            self.lock.reject();
            return None;
        };
        estimate.lag_frames += minimum;
        estimate.fractional_lag += minimum as f64;
        let change = self
            .lock
            .observe(latest, estimate, timestamp_offset, tracked)?;
        self.lag = estimate.lag_frames;
        self.drift = DriftTracker::new(&self.config);
        self.hop_frame = None;
        self.feedback_origin = None;
        self.measurement_frame = middle as f64 + estimate.fractional_lag;
        self.direct_reference = None;
        self.residual = None;
        Some(change)
    }
    /// Previewing lets the caller wait for the matching reference without advancing the filter.
    pub fn preview_hop(&self, microphone_frame: u64) -> Option<(i64, f64)> {
        if !self.locked() {
            return None;
        }
        let elapsed = self
            .hop_frame
            .map_or(Some(self.config.hop as u64), |previous| {
                microphone_frame
                    .checked_sub(previous)
                    .filter(|elapsed| *elapsed > 0)
            })?;
        let phase = self.drift.fraction() + self.drift.ppm() * 1e-6 * elapsed as f64;
        let whole = (phase + 0.5).floor() as i64;
        Some((self.lag.checked_add(whole)?, phase - whole as f64))
    }
    /// Call once per available hop, before applying its phase ramp; skip feedback on invalid hops.
    pub fn begin_hop(&mut self, microphone_frame: u64) -> Option<(i64, f64)> {
        if !self.locked() {
            return None;
        }
        let elapsed = if let Some(previous) = self.hop_frame {
            if microphone_frame <= previous {
                return None;
            }
            usize::try_from(microphone_frame - previous).ok()?
        } else {
            self.config.hop
        };
        self.drift.predict(elapsed);
        self.lag += self.drift.fold();
        self.applied = self.drift.fraction();
        self.hop_frame = Some(microphone_frame);
        self.feedback_origin.get_or_insert(microphone_frame);
        Some((self.lag, self.applied))
    }
    pub fn observe_fast(&mut self, direct: DirectPeak, mode_changed: bool) {
        if mode_changed {
            self.direct_reference = None;
        }
        if self.hop_frame.is_none() || direct.energy <= 1e-12 || !direct.position.is_finite() {
            return;
        }
        let reference = *self.direct_reference.get_or_insert(direct.position);
        self.residual = Some(direct.position - reference);
        self.drift
            .observe(self.applied, direct.position, reference, direct.variance);
    }
}
