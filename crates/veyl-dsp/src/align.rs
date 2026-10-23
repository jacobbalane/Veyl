//! GCC-PHAT follows Knapp and Carter (1976), doi:10.1109/TASSP.1976.1162830.
use crate::config::EngineConfig;
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::{ops::RangeInclusive, sync::Arc};
mod controller;
pub use controller::Aligner;

#[derive(Clone, Copy, Debug)]
pub struct LagEstimate {
    pub lag_frames: i64,
    pub fractional_lag: f64,
    pub psr: f32,
    pub reference_band_dbfs: f32,
}
struct Plans {
    size: usize,
    forward: Arc<dyn Fft<f32>>,
    inverse: Arc<dyn Fft<f32>>,
}
pub struct GccPhat {
    rate: u32,
    reference_frames: usize,
    max_microphone_frames: usize,
    ramp: usize,
    f_lo: f32,
    f_hi: f32,
    min_dbfs: f32,
    tolerance: f64,
    peak_ratio: f32,
    plans: Vec<Plans>,
    x: Vec<Complex<f32>>,
    y: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
}
impl GccPhat {
    pub fn new(
        rate: u32,
        config: &EngineConfig,
        search_width: usize,
    ) -> Result<Self, &'static str> {
        config.validate()?;
        if ![44100, 48000].contains(&rate) || search_width > rate as usize * 2 {
            return Err("Invalid alignment dimensions.");
        }
        let reference_frames = (rate as f32 * config.align_seconds).round() as usize;
        let max_microphone_frames = reference_frames + search_width;
        let smallest = (reference_frames * 2).next_power_of_two();
        let largest = (reference_frames + max_microphone_frames).next_power_of_two();
        let mut planner = FftPlanner::new();
        let mut plans = vec![];
        let mut size = smallest;
        loop {
            plans.push(Plans {
                size,
                forward: planner.plan_fft_forward(size),
                inverse: planner.plan_fft_inverse(size),
            });
            if size == largest {
                break;
            }
            size *= 2;
        }
        let scratch = plans
            .iter()
            .map(|plan| {
                plan.forward
                    .get_inplace_scratch_len()
                    .max(plan.inverse.get_inplace_scratch_len())
            })
            .max()
            .unwrap();
        Ok(Self {
            rate,
            reference_frames,
            max_microphone_frames,
            ramp: (rate as f32 * config.align_ramp_ms / 1000.0).round() as usize,
            f_lo: config.f_lo,
            f_hi: config.f_hi.min(0.45 * rate as f32),
            min_dbfs: config.align_min_dbfs,
            tolerance: config.align_tolerance_frames,
            peak_ratio: config.align_peak_ratio,
            plans,
            x: vec![Complex::default(); largest],
            y: vec![Complex::default(); largest],
            scratch: vec![Complex::default(); scratch],
        })
    }
    pub fn reference_frames(&self) -> usize {
        self.reference_frames
    }
    /// The caller supplies each window's origin and adjusts the returned lag by their difference.
    pub fn estimate(
        &mut self,
        reference: &[f32],
        microphone: &[f32],
        allowed: RangeInclusive<i64>,
    ) -> Option<LagEstimate> {
        self.estimate_near(reference, microphone, allowed, None)
    }
    pub fn estimate_near(
        &mut self,
        reference: &[f32],
        microphone: &[f32],
        allowed: RangeInclusive<i64>,
        expected: Option<f64>,
    ) -> Option<LagEstimate> {
        if reference.len() != self.reference_frames
            || microphone.len() < reference.len()
            || microphone.len() > self.max_microphone_frames
            || allowed.is_empty()
            || *allowed.start() <= -(reference.len() as i64)
            || *allowed.end() >= microphone.len() as i64
            || reference
                .iter()
                .chain(microphone)
                .any(|sample| !sample.is_finite() || sample.abs() > 8.0)
        {
            return None;
        }
        let size = (reference.len() + microphone.len()).next_power_of_two();
        let plan = self.plans.iter().find(|plan| plan.size == size)?;
        let x = &mut self.x[..size];
        let y = &mut self.y[..size];
        x.fill(Complex::default());
        y.fill(Complex::default());
        for (signal, output) in [(reference, &mut *x), (microphone, &mut *y)] {
            for (i, &sample) in signal.iter().enumerate() {
                let edge = i.min(signal.len() - 1 - i);
                let taper = if edge < self.ramp {
                    0.5 - 0.5 * (std::f32::consts::PI * edge as f32 / self.ramp as f32).cos()
                } else {
                    1.0
                };
                output[i].re = sample * taper;
            }
        }
        plan.forward.process_with_scratch(x, &mut self.scratch);
        plan.forward.process_with_scratch(y, &mut self.scratch);
        let mut power = 0.0_f64;
        for bin in 0..size {
            let hz = bin.min(size - bin) as f32 * self.rate as f32 / size as f32;
            if (self.f_lo..=self.f_hi).contains(&hz) {
                power += x[bin].norm_sqr() as f64;
                let cross = y[bin] * x[bin].conj();
                x[bin] = cross / (cross.norm() + 1e-12);
            } else {
                x[bin] = Complex::default();
            }
        }
        let level = (10.0 * (power / (size * reference.len()) as f64).max(1e-12).log10()) as f32;
        if level < self.min_dbfs {
            return None;
        }
        plan.inverse.process_with_scratch(x, &mut self.scratch);
        let at = |lag: i64| x[lag.rem_euclid(size as i64) as usize].re.abs();
        let global = allowed.clone().max_by(|a, b| at(*a).total_cmp(&at(*b)))?;
        // Two equally strong microphone paths can exchange rank as their clocks drift.
        // Keep a measured local peak near the previous path, provided it remains strong.
        let peak = expected
            .filter(|value| value.is_finite())
            .and_then(|expected| {
                let first = (expected - self.tolerance)
                    .ceil()
                    .max(*allowed.start() as f64) as i64;
                let last = (expected + self.tolerance)
                    .floor()
                    .min(*allowed.end() as f64) as i64;
                (first..=last)
                    .filter(|&lag| {
                        at(lag) >= at(lag - 1)
                            && at(lag) >= at(lag + 1)
                            && at(lag) >= self.peak_ratio * at(global)
                    })
                    .max_by(|a, b| at(*a).total_cmp(&at(*b)))
            })
            .unwrap_or(global);
        let amplitude = at(peak);
        let mut side = 0.0_f64;
        let mut count = 0;
        for lag in allowed {
            if lag.abs_diff(peak) > 40 {
                side += at(lag) as f64;
                count += 1;
            }
        }
        if count == 0 || amplitude <= 1e-12 {
            return None;
        }
        let a = at(peak - 1);
        let b = at(peak + 1);
        let denominator = a - 2.0 * amplitude + b;
        let fraction = if denominator.abs() > 1e-12 {
            (0.5 * (a - b) / denominator).clamp(-0.5, 0.5)
        } else {
            0.0
        };
        Some(LagEstimate {
            lag_frames: peak,
            fractional_lag: peak as f64 + fraction as f64,
            psr: (amplitude as f64 / (side / count as f64).max(1e-12)) as f32,
            reference_band_dbfs: level,
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub enum LockChange {
    Locked(LagEstimate),
    Relocked(LagEstimate),
}
#[derive(Clone, Copy)]
struct Candidate {
    frame: u64,
    value: f64,
}
pub struct AlignmentLock {
    config: EngineConfig,
    rate: u32,
    locked: bool,
    pending: Option<Candidate>,
    last_check: Option<u64>,
}
impl AlignmentLock {
    pub fn new(rate: u32, config: EngineConfig) -> Result<Self, &'static str> {
        config.validate()?;
        if ![44100, 48000].contains(&rate) {
            return Err("Invalid alignment rate.");
        }
        Ok(Self {
            config,
            rate,
            locked: false,
            pending: None,
            last_check: None,
        })
    }
    pub fn locked(&self) -> bool {
        self.locked
    }
    pub fn expected_lag(&self, timestamp_offset: f64, tracked_lag: Option<f64>) -> Option<f64> {
        if self.locked {
            tracked_lag
        } else {
            self.pending.map(|pending| pending.value + timestamp_offset)
        }
    }
    pub fn reset(&mut self) {
        self.locked = false;
        self.pending = None;
        self.last_check = None;
    }
    fn reject(&mut self) {
        self.pending = None;
    }
    pub fn observe(
        &mut self,
        frame: u64,
        estimate: LagEstimate,
        timestamp_offset: f64,
        tracked_lag: Option<f64>,
    ) -> Option<LockChange> {
        if !estimate.psr.is_finite()
            || estimate.psr < self.config.align_min_psr
            || !estimate.reference_band_dbfs.is_finite()
            || estimate.reference_band_dbfs < self.config.align_min_dbfs
            || !estimate.fractional_lag.is_finite()
            || !timestamp_offset.is_finite()
        {
            self.pending = None;
            return None;
        }
        let interval = (self.rate as f32
            * if self.locked {
                self.config.recheck_seconds
            } else {
                self.config.align_interval_seconds
            }) as u64;
        if self
            .last_check
            .is_some_and(|last| frame.saturating_sub(last) < interval)
        {
            return None;
        }
        self.last_check = Some(frame);
        // Compare acoustic delay at startup, so timestamp-observed clock drift cannot prevent lock.
        let value = if self.locked {
            estimate.fractional_lag - tracked_lag?
        } else {
            estimate.fractional_lag - timestamp_offset
        };
        if self.locked && value.abs() < self.config.align_tolerance_frames {
            self.pending = None;
            return None;
        }
        let accepted = self.pending.is_some_and(|old| {
            frame.saturating_sub(old.frame) >= interval
                && (value - old.value).abs() <= self.config.align_tolerance_frames
        });
        if accepted {
            let relocked = self.locked;
            self.locked = true;
            self.pending = None;
            Some(if relocked {
                LockChange::Relocked(estimate)
            } else {
                LockChange::Locked(estimate)
            })
        } else {
            self.pending = Some(Candidate { frame, value });
            None
        }
    }
}
pub struct DriftTracker {
    phi: f64,
    rho: f64,
    p00: f64,
    p01: f64,
    p11: f64,
    phase_noise: f64,
    rate_noise: f64,
}
impl DriftTracker {
    pub fn new(config: &EngineConfig) -> Self {
        Self {
            phi: 0.0,
            rho: 0.0,
            p00: 1.0,
            p01: 0.0,
            p11: 1e-6,
            phase_noise: config.drift_phase_noise,
            rate_noise: config.drift_rate_noise,
        }
    }
    pub fn fraction(&self) -> f64 {
        self.phi
    }
    pub fn ppm(&self) -> f64 {
        self.rho * 1e6
    }
    pub fn predict(&mut self, frames: usize) {
        let dt = frames as f64;
        self.phi += self.rho * dt;
        self.p00 += 2.0 * dt * self.p01 + dt * dt * self.p11 + self.phase_noise;
        self.p01 += dt * self.p11;
        self.p11 += self.rate_noise;
    }
    pub fn observe(
        &mut self,
        applied_fraction: f64,
        fast_direct: f64,
        reference_direct: f64,
        variance: f64,
    ) {
        let measurement = applied_fraction + fast_direct - reference_direct;
        if !measurement.is_finite() || !variance.is_finite() || variance <= 0.0 {
            return;
        }
        let innovation = measurement - self.phi;
        let total = self.p00 + variance.max(1e-6);
        let k0 = self.p00 / total;
        let k1 = self.p01 / total;
        let covariance = self.p01;
        self.phi += k0 * innovation;
        self.rho += k1 * innovation;
        self.p00 *= 1.0 - k0;
        self.p01 *= 1.0 - k0;
        self.p11 = (self.p11 - k1 * covariance).max(1e-20);
    }
    /// Folding changes only the integer origin; total applied delay and drift remain continuous.
    pub fn fold(&mut self) -> i64 {
        let frames = (self.phi + 0.5).floor() as i64;
        self.phi -= frames as f64;
        frames
    }
}
#[cfg(test)]
mod tests;
