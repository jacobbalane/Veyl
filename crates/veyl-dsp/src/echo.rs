//! Fast multichannel transfer estimates, activity masks and analytic echo maps.
mod gate;
mod maps;
use crate::{config::EngineConfig, StreamFormat};
use gate::ImpulseGate;
pub use maps::{direct_peak, DirectPeak, MapTransform};
pub(crate) use maps::{direct_peak_near, nearest_direct};
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::{f32::consts::TAU, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceMode {
    Mono,
    Stereo,
}
#[derive(Clone, Copy, Debug)]
pub struct EchoFrame {
    pub mode: ReferenceMode,
    pub mode_changed: bool,
    pub coherence: f32,
    pub bandwidth_hz: f32,
    pub reference_weights: [f32; 2],
    pub reference_cross: Complex<f32>,
    pub path_mismatch: f32,
    pub predicted_power: f64,
    pub residual_power: f64,
    pub impulse: bool,
    pub impulse_fraction: f32,
    pub direct: DirectPeak,
}

pub struct EchoEstimator {
    config: EngineConfig,
    format: StreamFormat,
    n: usize,
    bins: usize,
    refs: usize,
    mics: usize,
    mode: ReferenceMode,
    alpha: f32,
    coherence_alpha: f32,
    forward: Arc<dyn Fft<f32>>,
    scratch: Vec<Complex<f32>>,
    window: Vec<f32>,
    x: Vec<Complex<f32>>,
    y: Vec<Complex<f32>>,
    phase: Vec<Complex<f32>>,
    coherence_left: Vec<f32>,
    coherence_right: Vec<f32>,
    coherence_cross: Vec<Complex<f32>>,
    xx: Vec<f32>,
    lr: Vec<Complex<f32>>,
    yx: Vec<Complex<f32>>,
    transfer: Vec<Complex<f32>>,
    maps: Vec<Complex<f32>>,
    transform: MapTransform,
    band: Vec<f32>,
    activity: Vec<f32>,
    weights: Vec<f32>,
    gate: ImpulseGate,
    previous_direct: Option<f64>,
    path_average: Vec<Complex<f32>>,
    path_frames: u64,
    single_path: bool,
    power_alpha: f32,
    power: Vec<f32>,
    power_cross: Vec<Complex<f32>>,
    power_hops: u32,
    mean_power: Vec<f32>,
    mean_hops: u32,
    /// This hop's reference then microphone samples, kept only for short change measurements.
    signals: Vec<f32>,
    fraction: f64,
}
impl EchoEstimator {
    pub fn new(config: EngineConfig, format: StreamFormat) -> Result<Self, &'static str> {
        config.validate()?;
        format.validate()?;
        let n = config.window;
        let bins = n / 2 + 1;
        let refs = format.ref_channels as usize;
        let mics = format.mic_channels as usize;
        let forward = FftPlanner::new().plan_fft_forward(n);
        let scratch = vec![Complex::default(); forward.get_inplace_scratch_len()];
        let window = (0..n)
            .map(|i| 0.5 - 0.5 * (TAU * i as f32 / n as f32).cos())
            .collect();
        let high = config.f_hi.min(0.45 * format.sample_rate as f32);
        let ramp = |position: f32| {
            let phase = (position / config.edge_hz + 0.5).clamp(0.0, 1.0);
            0.5 - 0.5 * (std::f32::consts::PI * phase).cos()
        };
        let band = (0..bins)
            .map(|bin| {
                let hz = bin as f32 * format.sample_rate as f32 / n as f32;
                ramp(hz - config.f_lo) * ramp(high - hz)
            })
            .collect();
        let gate = ImpulseGate::new(
            (2 * format.sample_rate as usize).div_ceil(config.hop),
            config.impulse_ratio,
            config.impulse_hold_hops,
        );
        let transform = MapTransform::new(&config)?;
        Ok(Self {
            alpha: 1.0
                - (-(config.hop as f32) / (config.fast_seconds * format.sample_rate as f32)).exp(),
            coherence_alpha: 1.0
                - (-(config.hop as f32) / (config.coherence_seconds * format.sample_rate as f32))
                    .exp(),
            mode: ReferenceMode::Mono,
            n,
            bins,
            refs,
            mics,
            forward,
            scratch,
            window,
            x: vec![Complex::default(); n * refs],
            y: vec![Complex::default(); n * mics],
            phase: vec![Complex::default(); bins],
            coherence_left: vec![0.0; bins],
            coherence_right: vec![0.0; bins],
            coherence_cross: vec![Complex::default(); bins],
            xx: vec![0.0; bins * refs],
            lr: vec![Complex::default(); bins],
            yx: vec![Complex::default(); bins * refs * mics],
            transfer: vec![Complex::default(); bins * refs * mics],
            maps: vec![Complex::default(); (config.pre_taps + config.taps) * refs * mics],
            activity: vec![0.0; bins],
            weights: vec![0.0; bins],
            transform,
            band,
            gate,
            previous_direct: None,
            path_average: vec![Complex::default(); bins * refs * mics],
            path_frames: 0,
            single_path: false,
            power_alpha: 1.0
                - (-(config.hop as f32)
                    / (config.reference_power_seconds * format.sample_rate as f32))
                    .exp(),
            power: vec![0.0; bins * 2],
            power_cross: vec![Complex::default(); bins],
            power_hops: 0,
            mean_power: vec![0.0; bins],
            mean_hops: 0,
            signals: if config.residual_motion && config.change_window > 0 {
                vec![0.0; n * (refs + mics)]
            } else {
                vec![]
            },
            fraction: 0.0,
            config,
            format,
        })
    }
    pub fn bins(&self) -> usize {
        self.bins
    }
    /// Fixed pair layout: (microphone * format.ref_channels + reference), then frequency bin.
    /// In Mono mode, only reference slot zero is used; the other slots contain zeros.
    pub fn transfer(&self) -> &[Complex<f32>] {
        &self.transfer
    }
    /// The same pair layout as transfer(), with taps -P .. K-1 for each pair.
    pub fn maps(&self) -> &[Complex<f32>] {
        &self.maps
    }
    pub fn weights(&self) -> &[f32] {
        &self.weights
    }
    pub fn reset_averages(&mut self) {
        self.xx.fill(0.0);
        self.lr.fill(Complex::default());
        self.yx.fill(Complex::default());
        self.transfer.fill(Complex::default());
        self.maps.fill(Complex::default());
        self.activity.fill(0.0);
        self.weights.fill(0.0);
        self.gate.reset();
        self.previous_direct = None;
    }
    pub fn reset(&mut self) {
        self.reset_averages();
        self.path_average.fill(Complex::default());
        self.path_frames = 0;
        self.single_path = false;
        self.coherence_left.fill(0.0);
        self.coherence_right.fill(0.0);
        self.coherence_cross.fill(Complex::default());
        self.mode = ReferenceMode::Mono;
        self.power.fill(0.0);
        self.power_cross.fill(Complex::default());
        self.power_hops = 0;
        self.mean_power.fill(0.0);
        self.mean_hops = 0;
    }
    pub(crate) fn channels(&self) -> usize {
        if self.mode == ReferenceMode::Mono {
            1
        } else {
            2
        }
    }
    pub(crate) fn band(&self, bin: usize) -> f32 {
        self.band[bin]
    }
    pub(crate) fn microphone(&self, channel: usize, bin: usize) -> Complex<f32> {
        self.y[channel * self.n + bin]
    }
    /// The mean of all reference channels, whatever the current transfer mode.
    pub(crate) fn mean_reference(&self, bin: usize) -> Complex<f32> {
        if self.refs == 2 {
            (self.x[bin] + self.x[self.n + bin]) * 0.5
        } else {
            self.x[bin]
        }
    }
    /// Long-term power of each reference in the current transfer mode.
    pub(crate) fn long_term_power(&self, channel: usize, bin: usize) -> f32 {
        self.power[channel * self.bins + bin]
    }
    /// Long-term cross power of the Stereo references, left times conjugate right.
    pub(crate) fn long_term_cross(&self, bin: usize) -> Complex<f32> {
        self.power_cross[bin]
    }
    pub(crate) fn long_term_mean_power(&self, bin: usize) -> f32 {
        self.mean_power[bin]
    }
    /// Running means warm the long-term spectra up before the exponential average takes over.
    fn update_long_term_power(&mut self) {
        self.power_hops = self.power_hops.saturating_add(1);
        self.mean_hops = self.mean_hops.saturating_add(1);
        let alpha = self.power_alpha.max(1.0 / self.power_hops as f32);
        let mean_alpha = self.power_alpha.max(1.0 / self.mean_hops as f32);
        for bin in 0..self.bins {
            for channel in 0..self.channels() {
                let power = self.reference(channel, bin).norm_sqr();
                let average = &mut self.power[channel * self.bins + bin];
                *average += alpha * (power - *average);
            }
            if self.mode == ReferenceMode::Stereo {
                let cross = self.reference(0, bin) * self.reference(1, bin).conj();
                let average = self.power_cross[bin];
                self.power_cross[bin] = average + alpha * (cross - average);
            }
            let power = self.mean_reference(bin).norm_sqr();
            self.mean_power[bin] += mean_alpha * (power - self.mean_power[bin]);
        }
    }
    /// The window's samples of one reference input channel, before the analysis window.
    pub(crate) fn reference_signal(&self, channel: usize) -> &[f32] {
        &self.signals[channel * self.n..(channel + 1) * self.n]
    }
    pub(crate) fn microphone_signal(&self, channel: usize) -> &[f32] {
        let start = (self.refs + channel) * self.n;
        &self.signals[start..start + self.n]
    }
    pub(crate) fn reference_channels(&self) -> usize {
        self.refs
    }
    /// The fractional reference delay applied to this hop's spectra, in frames.
    pub(crate) fn fractional_delay(&self) -> f64 {
        self.fraction
    }
    pub(crate) fn reference(&self, channel: usize, bin: usize) -> Complex<f32> {
        if self.mode == ReferenceMode::Mono && self.refs == 2 {
            (self.x[bin] + self.x[self.n + bin]) * 0.5
        } else {
            self.x[channel * self.n + bin]
        }
    }
    /// Inputs contain N consecutive samples for each channel, in channel-major order.
    /// Background uses transfer()'s fixed layout. Enable the impulse gate after one second Running.
    pub fn process(
        &mut self,
        reference: &[f32],
        microphone: &[f32],
        fractional_delay: f64,
        background: Option<&[Complex<f32>]>,
        impulse_enabled: bool,
    ) -> Result<EchoFrame, &'static str> {
        if reference.len() != self.n * self.refs
            || microphone.len() != self.n * self.mics
            || background.is_some_and(|bg| bg.len() != self.transfer.len())
            || !fractional_delay.is_finite()
            || fractional_delay.abs() > 0.5
            || reference
                .iter()
                .chain(microphone)
                .any(|v| !v.is_finite() || v.abs() > 8.0)
        {
            return Err("Invalid echo-estimation window.");
        }
        if !self.signals.is_empty() {
            let (references, microphones) = self.signals.split_at_mut(reference.len());
            references.copy_from_slice(reference);
            microphones.copy_from_slice(microphone);
        }
        self.fraction = fractional_delay;
        for (input, output) in [(reference, &mut self.x), (microphone, &mut self.y)] {
            for (signal, spectrum) in input.chunks(self.n).zip(output.chunks_mut(self.n)) {
                for i in 0..self.n {
                    spectrum[i] = Complex::new(signal[i] * self.window[i], 0.0);
                }
                self.forward
                    .process_with_scratch(spectrum, &mut self.scratch);
            }
        }
        for bin in 0..self.bins {
            self.phase[bin] = Complex::from_polar(
                1.0,
                (-std::f64::consts::TAU * bin as f64 * fractional_delay / self.n as f64) as f32,
            );
        }
        for channel in self.x.chunks_mut(self.n) {
            for (bin, phase) in self.phase.iter().enumerate() {
                channel[bin] *= phase;
            }
        }
        let (predicted_power, residual_power) = self.prediction(background);
        // A louder moment of the music raises the model's error with it; a click or knock is
        // sound the reference did not cause, so it raises the residual relative to the prediction.
        let impulse = self.gate.observe(
            if self.config.impulse_relative {
                residual_power / predicted_power.max(1e-30)
            } else {
                residual_power
            },
            impulse_enabled,
        );
        let previous = self.mode;
        let coherence = if !impulse {
            self.update_coherence()
        } else {
            self.coherence()
        };
        if self.refs == 2 && !impulse {
            if self.single_path {
                self.mode = ReferenceMode::Mono;
            } else if self.mode == ReferenceMode::Mono && coherence < self.config.mono_to_stereo {
                self.mode = ReferenceMode::Stereo;
            } else if self.mode == ReferenceMode::Stereo && coherence > self.config.stereo_to_mono {
                self.mode = ReferenceMode::Mono;
            }
        }
        let mode_changed = previous != self.mode;
        if mode_changed {
            self.reset_averages();
            // The mean reference keeps its meaning; the per-mode spectra do not.
            self.power.fill(0.0);
            self.power_cross.fill(Complex::default());
            self.power_hops = 0;
        }
        if !impulse {
            if self.config.residual_motion {
                self.update_long_term_power();
            }
            self.average();
            self.estimate_transfer();
            self.update_mask();
            self.transform
                .render(&self.transfer, &self.weights, &mut self.maps)?;
        }
        let mut powers = [0.0; 2];
        for (channel, power) in powers.iter_mut().enumerate().take(self.channels()) {
            *power = self.xx[channel * self.bins..(channel + 1) * self.bins]
                .iter()
                .zip(&self.band)
                .map(|(value, band)| value * band)
                .sum::<f32>();
        }
        let total = powers.iter().sum::<f32>().max(1e-20);
        let reference_weights = if self.mode == ReferenceMode::Mono {
            [1.0, 0.0]
        } else {
            [powers[0] / total, powers[1] / total]
        };
        let direct = maps::direct_peak_near(
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
        let mut reference_cross = if self.mode == ReferenceMode::Stereo {
            self.lr
                .iter()
                .zip(&self.band)
                .map(|(value, band)| *value * *band)
                .sum::<Complex<f32>>()
                / total
        } else {
            Complex::default()
        };
        let bound = (reference_weights[0] * reference_weights[1]).sqrt();
        if reference_cross.norm() > bound && reference_cross.norm() > 0.0 {
            reference_cross *= bound / reference_cross.norm();
        }
        let mut difference = 0.0;
        let mut path_power = 0.0;
        if self.mode == ReferenceMode::Stereo && !impulse {
            self.path_frames += self.config.hop as u64;
            for (average, value) in self.path_average.iter_mut().zip(&self.transfer) {
                *average += 0.01 * (*value - *average);
            }
            for c in 0..self.mics {
                for bin in 0..self.bins {
                    let left = self.path_average[c * self.refs * self.bins + bin];
                    let right = self.path_average[(c * self.refs + 1) * self.bins + bin];
                    difference += (left - right).norm_sqr() * self.band[bin];
                    path_power += (left.norm_sqr() + right.norm_sqr()) * self.band[bin];
                }
            }
        }
        let path_mismatch = difference / path_power.max(1e-20);
        if self.config.infer_single_path
            && self.path_frames >= self.format.sample_rate as u64
            && path_power > 1e-12
            && path_mismatch < 0.01
        {
            self.single_path = true;
        }
        Ok(EchoFrame {
            reference_cross,
            path_mismatch,
            mode: self.mode,
            mode_changed,
            coherence,
            bandwidth_hz: self.weights.iter().sum::<f32>() * self.format.sample_rate as f32
                / self.n as f32,
            reference_weights,
            predicted_power: if mode_changed { 0.0 } else { predicted_power },
            residual_power,
            impulse,
            impulse_fraction: self.gate.fraction(),
            direct,
        })
    }
    fn prediction(&self, background: Option<&[Complex<f32>]>) -> (f64, f64) {
        let mut predicted = 0.0;
        let mut residual = 0.0;
        for c in 0..self.mics {
            for bin in 1..self.bins - 1 {
                if self.band[bin] == 0.0 {
                    continue;
                }
                let mut value = Complex::default();
                if let Some(bg) = background {
                    for s in 0..self.channels() {
                        value += bg[(c * self.refs + s) * self.bins + bin] * self.reference(s, bin);
                    }
                }
                predicted += (value.norm_sqr() * self.band[bin]) as f64;
                residual += ((self.y[c * self.n + bin] - value).norm_sqr() * self.band[bin]) as f64;
            }
        }
        (predicted, residual)
    }
    fn update_coherence(&mut self) -> f32 {
        if self.refs == 1 {
            return 1.0;
        }
        for bin in 0..self.bins {
            let left = self.x[bin];
            let right = self.x[self.n + bin];
            self.coherence_left[bin] +=
                self.coherence_alpha * (left.norm_sqr() - self.coherence_left[bin]);
            self.coherence_right[bin] +=
                self.coherence_alpha * (right.norm_sqr() - self.coherence_right[bin]);
            self.coherence_cross[bin] = self.coherence_cross[bin]
                + self.coherence_alpha * (left * right.conj() - self.coherence_cross[bin]);
        }
        self.coherence()
    }
    fn coherence(&self) -> f32 {
        if self.refs == 1 {
            return 1.0;
        }
        let threshold = self
            .coherence_left
            .iter()
            .zip(&self.coherence_right)
            .zip(&self.band)
            .filter(|(_, band)| **band > 0.0)
            .map(|((l, r), _)| l + r)
            .fold(0.0_f32, f32::max)
            * 10.0_f32.powf(-self.config.activity_db / 10.0);
        let mut sum = 0.0;
        let mut count = 0;
        for bin in 0..self.bins {
            let l = self.coherence_left[bin];
            let r = self.coherence_right[bin];
            if self.band[bin] > 0.0 && l + r > threshold && l * r > 1e-20 {
                sum += (self.coherence_cross[bin].norm_sqr() / (l * r)).clamp(0.0, 1.0);
                count += 1;
            }
        }
        if count == 0 {
            1.0
        } else {
            sum / count as f32
        }
    }
    fn average(&mut self) {
        for bin in 0..self.bins {
            for s in 0..self.channels() {
                let x = self.reference(s, bin);
                let power = &mut self.xx[s * self.bins + bin];
                *power += self.alpha * (x.norm_sqr() - *power);
                for c in 0..self.mics {
                    let cross = &mut self.yx[(c * self.refs + s) * self.bins + bin];
                    *cross += self.alpha * (self.y[c * self.n + bin] * x.conj() - *cross);
                }
            }
            if self.mode == ReferenceMode::Stereo {
                let cross = self.reference(0, bin) * self.reference(1, bin).conj();
                self.lr[bin] = self.lr[bin] + self.alpha * (cross - self.lr[bin]);
            }
        }
    }
    fn estimate_transfer(&mut self) {
        let channels = self.channels();
        let mut power = 0.0;
        let mut count = 0;
        for bin in 0..self.bins {
            if self.band[bin] > 0.0 {
                for s in 0..channels {
                    power += self.xx[s * self.bins + bin] as f64;
                    count += 1;
                }
            }
        }
        let delta = self.config.regularization as f64 * power / count.max(1) as f64 + 1e-12;
        for c in 0..self.mics {
            for bin in 0..self.bins {
                let pair = (c * self.refs) * self.bins + bin;
                if channels == 1 {
                    self.transfer[pair] = self.yx[pair] / (self.xx[bin] as f64 + delta) as f32;
                } else {
                    // Double precision keeps the regularized Hermitian inverse stable.
                    let a = self.xx[bin] as f64 + delta;
                    let d = self.xx[self.bins + bin] as f64 + delta;
                    let widen = |v: Complex<f32>| Complex::new(v.re as f64, v.im as f64);
                    let narrow = |v: Complex<f64>| Complex::new(v.re as f32, v.im as f32);
                    let b = widen(self.lr[bin]);
                    let determinant = (a * d - b.norm_sqr()).max(delta * delta);
                    let left = widen(self.yx[pair]);
                    let right = widen(self.yx[pair + self.bins]);
                    self.transfer[pair] = narrow((left * d - right * b.conj()) / determinant);
                    self.transfer[pair + self.bins] = narrow((right * a - left * b) / determinant);
                }
            }
        }
    }
    fn update_mask(&mut self) {
        let maximum = (0..self.bins)
            .filter(|&bin| self.band[bin] > 0.0)
            .map(|bin| self.reference_power(bin))
            .fold(0.0_f32, f32::max);
        let threshold = maximum * 10.0_f32.powf(-self.config.activity_db / 10.0);
        for bin in 0..self.bins {
            self.activity[bin] = if maximum > 1e-20
                && self.band[bin] > 0.0
                && self.reference_power(bin) >= threshold
            {
                1.0
            } else {
                0.0
            };
        }
        let half = self.config.activity_bins / 2;
        for bin in 0..self.bins {
            let first = bin.saturating_sub(half);
            let last = (bin + half + 1).min(self.bins);
            self.weights[bin] = self.band[bin] * self.activity[first..last].iter().sum::<f32>()
                / self.config.activity_bins as f32;
        }
    }
    fn reference_power(&self, bin: usize) -> f32 {
        (0..self.channels())
            .map(|s| self.xx[s * self.bins + bin])
            .sum()
    }
}

#[cfg(test)]
mod tests;
