//! Normalize echo changes against a learned floor and follow a concentrated moving return.
use crate::{
    config::EngineConfig,
    types::{RangePreset, Sensitivity},
    StreamFormat,
};
use rustfft::num_complex::Complex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct MotionFeatures {
    pub frame: u64,
    pub valid: bool,
    pub settling: bool,
    pub z_max: f32,
    pub peak_floor: f32,
    pub peak_energy: f32,
    pub direct_energy: f32,
    pub peakiness: f32,
    pub mean_energy: f32,
    pub r_peak_cm: Option<f32>,
    pub distance_cm: Option<f32>,
    pub present: bool,
    pub changed: bool,
}
pub struct MotionInput<'a> {
    pub frame: u64,
    pub dynamic: &'a [Complex<f32>],
    pub normalized: Option<&'a [Complex<f32>]>,
    pub direct_position: f64,
    pub direct_energy: f32,
    pub reference_weights: [f32; 2],
    pub reference_cross: Complex<f32>,
    pub settling: bool,
    pub held: bool,
    /// The dynamic map already measures change, so settling learns its recent mean energy.
    pub energy_floor: bool,
}
pub struct MotionTracker {
    config: EngineConfig,
    refs: usize,
    mics: usize,
    taps: usize,
    cm_per_tap: f32,
    floor_alpha: f32,
    calibration_alpha: f32,
    calibration_mean: Vec<Complex<f32>>,
    calibration_variance: Vec<f32>,
    calibration_cross: Vec<Complex<f32>>,
    calibration_initialized: bool,
    distance_alpha: f32,
    z_on: f32,
    range: (f32, f32),
    floor: Vec<f32>,
    energy: Vec<f32>,
    z: Vec<f32>,
    smoothed: Vec<f32>,
    scratch: Vec<f32>,
    settling_count: u64,
    on_count: usize,
    off_count: usize,
    distances: [f32; 3],
    distance_count: usize,
    distance_index: usize,
    distance: Option<f32>,
    row: [f32; 60],
    last: MotionFeatures,
    /// Per microphone, its own direct sound's tap position, when known.
    own_directs: Vec<Option<f64>>,
    /// Consecutive hops with a much stronger echo outside the track gate.
    escape_hops: u32,
}
impl MotionTracker {
    pub fn new(config: EngineConfig, format: StreamFormat) -> Result<Self, &'static str> {
        config.validate()?;
        format.validate()?;
        let taps = config.pre_taps + config.taps;
        let alpha =
            |time: f32| 1.0 - (-(config.hop as f32) / (time * format.sample_rate as f32)).exp();
        Ok(Self {
            refs: format.ref_channels as usize,
            mics: format.mic_channels as usize,
            taps,
            cm_per_tap: (331.3 + 0.606 * config.temperature_c) * 100.0
                / (2.0 * format.sample_rate as f32),
            floor_alpha: alpha(config.floor_seconds),
            calibration_alpha: alpha(config.floor_calibration_seconds),
            calibration_mean: vec![
                Complex::default();
                taps * format.ref_channels as usize
                    * format.mic_channels as usize
            ],
            calibration_variance: vec![
                0.0;
                taps * format.ref_channels as usize
                    * format.mic_channels as usize
            ],
            calibration_initialized: false,
            calibration_cross: vec![Complex::default(); taps * format.mic_channels as usize],
            distance_alpha: alpha(config.distance_smooth_seconds),
            z_on: config.z_on,
            range: (config.range_min_cm, config.range_max_cm),
            floor: vec![0.0; taps],
            energy: vec![0.0; taps],
            z: vec![0.0; taps],
            smoothed: vec![0.0; taps],
            scratch: vec![0.0; taps],
            settling_count: 0,
            on_count: 0,
            off_count: 0,
            distances: [0.0; 3],
            distance_count: 0,
            distance_index: 0,
            distance: None,
            row: [0.0; 60],
            last: MotionFeatures::default(),
            own_directs: vec![None; format.mic_channels as usize],
            escape_hops: 0,
            config,
        })
    }
    /// A microphone farther from the speaker hears its own direct sound later, inside the echo
    /// range of the nearest pair. Model errors around that strong sound are not motion, so its
    /// direct region is left out of that microphone's motion energy.
    pub fn set_own_directs(&mut self, positions: &[Option<f64>]) {
        for (own, position) in self.own_directs.iter_mut().zip(positions) {
            *own = position.filter(|value| value.is_finite());
        }
    }
    pub fn present(&self) -> bool {
        self.last.present
    }
    pub fn features(&self) -> MotionFeatures {
        self.last
    }
    pub fn normalized_motion(&self) -> &[f32] {
        &self.z
    }
    pub fn floor(&self) -> &[f32] {
        &self.floor
    }
    pub fn row(&self) -> &[f32; 60] {
        &self.row
    }
    pub fn take_row(&mut self) -> [f32; 60] {
        let row = self.row;
        self.row.fill(0.0);
        row
    }
    fn reset_presence(&mut self) {
        self.escape_hops = 0;
        self.on_count = 0;
        self.off_count = 0;
        self.distance_count = 0;
        self.distance_index = 0;
        self.distance = None;
        self.last.present = false;
        self.last.distance_cm = None;
    }
    pub fn set_sensitivity(&mut self, preset: Sensitivity) {
        self.z_on = preset.z_on();
        self.reset_presence();
    }
    pub fn set_range(&mut self, preset: RangePreset) {
        self.range = preset.limits_cm();
        self.reset_presence();
    }
    pub fn reset(&mut self) {
        self.reset_presence();
        self.floor.fill(0.0);
        self.calibration_mean.fill(Complex::default());
        self.calibration_variance.fill(0.0);
        self.calibration_cross.fill(Complex::default());
        self.calibration_initialized = false;
        self.energy.fill(0.0);
        self.z.fill(0.0);
        self.smoothed.fill(0.0);
        self.settling_count = 0;
        self.row.fill(0.0);
        self.last = MotionFeatures::default();
    }
    pub fn process(&mut self, input: MotionInput<'_>) -> Result<MotionFeatures, &'static str> {
        let MotionInput {
            frame,
            dynamic,
            normalized,
            direct_position,
            direct_energy,
            reference_weights,
            reference_cross,
            settling,
            held,
            energy_floor,
        } = input;
        if dynamic.len() != self.taps * self.refs * self.mics
            || normalized.is_some_and(|samples| samples.len() != dynamic.len())
            || !direct_position.is_finite()
            || !direct_energy.is_finite()
            || direct_energy < 0.0
            || reference_weights
                .iter()
                .any(|weight| !weight.is_finite() || !(0.0..=1.0).contains(weight))
            || !reference_cross.re.is_finite()
            || !reference_cross.im.is_finite()
            || reference_cross.norm_sqr() > reference_weights[0] * reference_weights[1] + 1e-5
        {
            return Err("Invalid motion-map dimensions or energy.");
        }
        if held {
            self.last.frame = frame;
            self.last.valid = false;
            self.last.changed = false;
            return Ok(self.last);
        }
        self.energy.fill(0.0);
        for c in 0..self.mics {
            let excluded = self.own_directs[c].map(|position| {
                let centre = position + self.config.pre_taps as f64;
                (
                    centre - self.config.direct_before as f64,
                    centre + self.config.direct_after as f64,
                )
            });
            for k in 0..self.taps {
                if excluded.is_some_and(|(first, last)| (first..=last).contains(&(k as f64))) {
                    continue;
                }
                let mut change = [Complex::default(); 2];
                for (s, weight) in reference_weights.iter().enumerate().take(self.refs) {
                    let i = (c * self.refs + s) * self.taps + k;
                    let changed = dynamic[i].norm_sqr();
                    // A vanished background echo must not become the strongest moving return.
                    let supported =
                        normalized.map_or(changed, |current| changed.min(current[i].norm_sqr()));
                    self.energy[k] += weight * supported;
                    change[s] = dynamic[i]
                        * if changed > 1e-30 {
                            (supported / changed).sqrt()
                        } else {
                            0.0
                        };
                }
                if self.refs == 2 {
                    self.energy[k] += 2.0 * (reference_cross * change[0] * change[1].conj()).re;
                }
            }
        }
        if settling {
            self.settling_count += 1;
            if let Some(samples) = normalized {
                if self.calibration_initialized && self.refs == 2 {
                    for c in 0..self.mics {
                        for k in 0..self.taps {
                            let left = (c * self.refs) * self.taps + k;
                            let right = left + self.taps;
                            let cross = (samples[left] - self.calibration_mean[left])
                                * (samples[right] - self.calibration_mean[right]).conj();
                            let i = c * self.taps + k;
                            self.calibration_cross[i] = (1.0 - self.calibration_alpha)
                                * (self.calibration_cross[i] + self.calibration_alpha * cross);
                        }
                    }
                }
                for (i, value) in samples.iter().enumerate() {
                    if self.calibration_initialized {
                        let delta = *value - self.calibration_mean[i];
                        self.calibration_mean[i] += self.calibration_alpha * delta;
                        self.calibration_variance[i] = (1.0 - self.calibration_alpha)
                            * (self.calibration_variance[i]
                                + self.calibration_alpha * delta.norm_sqr());
                    } else {
                        self.calibration_mean[i] = *value;
                    }
                }
                self.calibration_initialized = true;
            }
        }
        let lower = (self.config.floor_relative * direct_energy).max(1e-20);
        for k in 0..self.taps {
            let distance =
                (k as f32 - self.config.pre_taps as f32 - direct_position as f32) * self.cm_per_tap;
            if !(0.0..60.0).contains(&distance) {
                self.energy[k] = 0.0;
            }
            if settling && energy_floor {
                self.floor[k] = if self.settling_count <= 1 {
                    self.energy[k]
                } else {
                    self.floor[k] + self.calibration_alpha * (self.energy[k] - self.floor[k])
                };
            } else if settling {
                if normalized.is_some() {
                    self.floor[k] = 0.0;
                    for c in 0..self.mics {
                        for (s, weight) in reference_weights.iter().enumerate().take(self.refs) {
                            self.floor[k] += weight
                                * self.calibration_variance[(c * self.refs + s) * self.taps + k];
                        }
                        if self.refs == 2 {
                            self.floor[k] += 2.0
                                * (reference_cross * self.calibration_cross[c * self.taps + k]).re;
                        }
                    }
                } else {
                    self.floor[k] += (self.energy[k] - self.floor[k]) / self.settling_count as f32;
                }
            } else if self.energy[k] / self.floor[k].max(lower)
                < self.z_on * self.config.floor_learn_fraction
            {
                self.floor[k] += self.floor_alpha * (self.energy[k] - self.floor[k]);
            }
            self.floor[k] = self.floor[k].max(lower);
            self.energy[k] = self.energy[k].max(0.0);
            self.z[k] = self.energy[k] / self.floor[k];
        }
        let mut count = 0;
        let mut best = None;
        let mut overall = None;
        let mut sum = 0.0;
        for k in 0..self.taps {
            let left = self.z[k.saturating_sub(1)];
            let right = self.z[(k + 1).min(self.taps - 1)];
            self.smoothed[k] = (left + 2.0 * self.z[k] + right) * 0.25;
            let distance =
                (k as f32 - self.config.pre_taps as f32 - direct_position as f32) * self.cm_per_tap;
            if !settling && (0.0..60.0).contains(&distance) {
                let index = distance.floor() as usize;
                self.row[index] =
                    self.row[index].max((10.0 * self.z[k].max(1.0).log10()).clamp(0.0, 30.0));
            }
            if (self.range.0..=self.range.1).contains(&distance) {
                self.scratch[count] = self.smoothed[k];
                count += 1;
                sum += self.smoothed[k];
                // A present hand moves only a few millimetres per hop, so its echo is sought
                // near where it was; a brief burst elsewhere cannot capture the track.
                let gated = match (self.last.present, self.distance) {
                    (true, Some(track)) if self.config.track_gate_cm > 0.0 => {
                        (distance - track).abs() <= self.config.track_gate_cm
                    }
                    _ => true,
                };
                if gated && best.is_none_or(|previous| self.smoothed[k] > self.smoothed[previous]) {
                    best = Some(k);
                }
                if overall.is_none_or(|previous| self.smoothed[k] > self.smoothed[previous]) {
                    overall = Some(k);
                }
            }
        }
        // A burst lasts a hop or two. A stronger echo outside the gate for longer is where the
        // hand went, for example after it left a spot the background still remembers.
        if let (Some(gated), Some(outside)) = (best, overall) {
            let escaping = outside != gated
                && self.smoothed[outside]
                    >= (self.config.track_escape_ratio * self.smoothed[gated]).max(self.z_on);
            self.escape_hops = if escaping { self.escape_hops + 1 } else { 0 };
            if self.escape_hops >= 3 {
                best = Some(outside);
                self.escape_hops = 0;
            }
        }
        self.scratch[..count].sort_unstable_by(f32::total_cmp);
        let median = if count == 0 {
            0.0
        } else if count.is_multiple_of(2) {
            (self.scratch[count / 2 - 1] + self.scratch[count / 2]) * 0.5
        } else {
            self.scratch[count / 2]
        };
        let z_max = best.map_or(0.0, |k| self.smoothed[k]);
        let peakiness = z_max / median.max(1e-12);
        let r_peak_cm = best.map(|k| {
            let left = self.smoothed[k.saturating_sub(1)];
            let right = self.smoothed[(k + 1).min(self.taps - 1)];
            let denominator = left - 2.0 * z_max + right;
            let fraction = if denominator.abs() > 1e-12 {
                (0.5 * (left - right) / denominator).clamp(-0.5, 0.5)
            } else {
                0.0
            };
            (k as f32 - self.config.pre_taps as f32 - direct_position as f32 + fraction)
                * self.cm_per_tap
        });
        let was_present = self.last.present;
        let mut present = was_present;
        if settling {
            self.reset_presence();
            present = false;
        } else if !present {
            self.on_count = if z_max >= self.z_on && peakiness >= self.config.peakiness {
                self.on_count + 1
            } else {
                0
            };
            if self.on_count >= self.config.presence_on_hops {
                present = true;
                self.off_count = 0;
            }
        } else {
            self.off_count = if z_max < self.z_on * 0.5 {
                self.off_count + 1
            } else {
                0
            };
            if self.off_count >= self.config.presence_off_hops {
                self.reset_presence();
                present = false;
            }
        }
        if present {
            if let Some(distance) = r_peak_cm {
                self.distances[self.distance_index] = distance;
                self.distance_index = (self.distance_index + 1) % 3;
                self.distance_count = (self.distance_count + 1).min(3);
                let mut sorted = self.distances;
                sorted[..self.distance_count].sort_unstable_by(f32::total_cmp);
                let value = sorted[self.distance_count / 2];
                self.distance = Some(
                    self.distance
                        .map_or(value, |old| old + self.distance_alpha * (value - old)),
                );
            }
        }
        self.last = MotionFeatures {
            frame,
            valid: true,
            settling,
            z_max,
            peak_floor: best.map_or(0.0, |k| self.floor[k]),
            peak_energy: best.map_or(0.0, |k| self.energy[k]),
            direct_energy,
            peakiness,
            mean_energy: sum / count.max(1) as f32,
            r_peak_cm,
            distance_cm: if present { self.distance } else { None },
            present,
            changed: was_present != present,
        };
        Ok(self.last)
    }
}

#[cfg(test)]
mod tests;
