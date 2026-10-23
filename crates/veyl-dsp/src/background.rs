//! Normalize the direct sound before learning stationary echoes.
use crate::{
    config::EngineConfig,
    echo::{direct_peak_near, DirectPeak, MapTransform},
    StreamFormat,
};
use rustfft::num_complex::Complex;

#[derive(Clone, Copy, Debug)]
pub struct BackgroundFrame {
    pub direct: DirectPeak,
    pub direct_energy: f32,
    pub settling: bool,
    pub model_jump: bool,
}
pub struct Background {
    config: EngineConfig,
    rate: u32,
    bins: usize,
    taps: usize,
    pairs: usize,
    transfer: Vec<Complex<f32>>,
    maps: Vec<Complex<f32>>,
    dynamic: Vec<Complex<f32>>,
    normalized: Vec<Complex<f32>>,
    normalized_maps: Vec<Complex<f32>>,
    gains: Vec<Complex<f32>>,
    previous_gains: Vec<Complex<f32>>,
    transform: MapTransform,
    initialized: bool,
    previous_direct: Option<f64>,
    settle_frames: u64,
    presence_frames: u64,
}
impl Background {
    pub fn new(config: EngineConfig, format: StreamFormat) -> Result<Self, &'static str> {
        config.validate()?;
        format.validate()?;
        let bins = config.window / 2 + 1;
        let taps = config.pre_taps + config.taps;
        let pairs = format.ref_channels as usize * format.mic_channels as usize;
        Ok(Self {
            rate: format.sample_rate,
            bins,
            taps,
            pairs,
            transfer: vec![Complex::default(); bins * pairs],
            maps: vec![Complex::default(); taps * pairs],
            dynamic: vec![Complex::default(); taps * pairs],
            normalized: vec![Complex::default(); bins * pairs],
            normalized_maps: vec![Complex::default(); taps * pairs],
            gains: vec![Complex::new(1.0, 0.0); pairs],
            previous_gains: vec![Complex::new(1.0, 0.0); pairs],
            transform: MapTransform::new(&config)?,
            initialized: false,
            previous_direct: None,
            settle_frames: (config.settle_seconds * format.sample_rate as f32).ceil() as u64,
            presence_frames: 0,
            config,
        })
    }
    pub fn transfer(&self) -> &[Complex<f32>] {
        &self.transfer
    }
    pub fn maps(&self) -> &[Complex<f32>] {
        &self.maps
    }
    pub fn dynamic(&self) -> &[Complex<f32>] {
        &self.dynamic
    }
    pub fn normalized_maps(&self) -> &[Complex<f32>] {
        &self.normalized_maps
    }
    pub fn settled(&self) -> bool {
        self.initialized && self.settle_frames == 0
    }
    pub fn reset(&mut self, short: bool) {
        self.transfer.fill(Complex::default());
        self.maps.fill(Complex::default());
        self.dynamic.fill(Complex::default());
        self.normalized.fill(Complex::default());
        self.normalized_maps.fill(Complex::default());
        self.gains.fill(Complex::new(1.0, 0.0));
        self.previous_gains.fill(Complex::new(1.0, 0.0));
        self.initialized = false;
        self.previous_direct = None;
        self.presence_frames = 0;
        self.settle_frames = (self.rate as f32
            * if short {
                self.config.reset_settle_seconds
            } else {
                self.config.settle_seconds
            })
        .ceil() as u64;
    }
    /// Model jumps leave the background untouched; the caller resets only the fast averages.
    pub fn process(
        &mut self,
        transfer: &[Complex<f32>],
        maps: &[Complex<f32>],
        weights: &[f32],
        hand_present: bool,
        held: bool,
    ) -> Result<BackgroundFrame, &'static str> {
        if transfer.len() != self.transfer.len()
            || maps.len() != self.maps.len()
            || weights.len() != self.bins
        {
            return Err("Invalid background dimensions.");
        }
        let settling = !self.settled();
        if !self.initialized {
            self.transfer.copy_from_slice(transfer);
            self.initialized = true;
        }
        self.transform
            .render(&self.transfer, weights, &mut self.maps)?;
        let direct = direct_peak_near(
            &self.maps,
            self.config.pre_taps,
            self.config.taps,
            self.previous_direct,
            self.config.align_tolerance_frames,
            self.config.align_peak_ratio,
        );
        if direct.energy > 1e-12 {
            self.previous_direct = Some(direct.position);
        }
        let centre = (direct.position.floor() as i64 + self.config.pre_taps as i64)
            .clamp(0, self.taps as i64 - 1) as usize;
        let first = centre.saturating_sub(self.config.direct_before);
        let end = (centre + self.config.direct_after + 1).min(self.taps);
        let mut direct_energy = 0.0;
        for pair in 0..self.pairs {
            let start = pair * self.taps;
            direct_energy += self.maps[start + first..start + end]
                .iter()
                .map(|tap| tap.norm_sqr())
                .sum::<f32>();
        }
        let result = |model_jump| BackgroundFrame {
            direct,
            direct_energy,
            settling,
            model_jump,
        };
        if held {
            return Ok(result(false));
        }
        let mut jump = false;
        for pair in 0..self.pairs {
            let start = pair * self.taps;
            let mut cross = Complex::default();
            let mut energy = 0.0;
            for k in first..end {
                cross += maps[start + k] * self.maps[start + k].conj();
                energy += self.maps[start + k].norm_sqr();
            }
            let gain = if energy > 1e-20 {
                cross / energy
            } else {
                Complex::new(1.0, 0.0)
            };
            let phase_step = (gain * self.previous_gains[pair].conj())
                .arg()
                .abs()
                .to_degrees();
            jump |= energy > 1e-20
                && (!(self.config.min_gain..=self.config.max_gain).contains(&gain.norm())
                    || phase_step > self.config.max_phase_step_degrees);
            self.gains[pair] = gain;
        }
        if jump && !settling {
            return Ok(result(true));
        }
        for pair in 0..self.pairs {
            let gain = if self.gains[pair].norm_sqr() > 1e-20 {
                self.gains[pair]
            } else {
                Complex::new(1.0, 0.0)
            };
            for bin in 0..self.bins {
                let i = pair * self.bins + bin;
                self.normalized[i] = transfer[i] / gain;
            }
            for k in 0..self.taps {
                let i = pair * self.taps + k;
                self.normalized_maps[i] = maps[i] / gain;
                self.dynamic[i] = self.normalized_maps[i] - self.maps[i];
            }
        }
        self.previous_gains.copy_from_slice(&self.gains);
        self.presence_frames = if hand_present {
            self.presence_frames.saturating_add(self.config.hop as u64)
        } else {
            0
        };
        if settling {
            // Measure differences before copying, so settling can initialize the motion floor.
            self.transfer.copy_from_slice(transfer);
            self.settle_frames = self.settle_frames.saturating_sub(self.config.hop as u64);
        } else {
            let time = if hand_present
                && self.presence_frames
                    <= (self.config.presence_hold_seconds * self.rate as f32) as u64
            {
                self.config.background_hold_seconds
            } else {
                self.config.background_seconds
            };
            let beta = 1.0 - (-(self.config.hop as f32) / (time * self.rate as f32)).exp();
            for (background, current) in self.transfer.iter_mut().zip(&self.normalized) {
                *background += beta * (*current - *background);
            }
        }
        Ok(result(false))
    }
}

#[cfg(test)]
mod tests;
