//! Measure change over a short, central part of the window.
//!
//! A hand moving toward the microphone shortens its echo's delay during the window. Over the
//! whole 43 ms window, a 50 cm/s approach turns the high band's phase by most of a cycle, so the
//! correlation cancels and the moving hand disappears from the map. A segment a quarter as
//! long keeps that echo sharp. The background is still learned over the whole window.
use crate::{config::EngineConfig, echo::EchoEstimator};
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::{f32::consts::TAU, sync::Arc};

/// Per microphone, the inputs the whole-window residual already measured.
pub(super) struct ShortInputs<'a> {
    /// Normalized background transfer, in the whole window's pair layout.
    pub transfer: &'a [Complex<f32>],
    /// Field gain per pair.
    pub gains: &'a [Complex<f32>],
    /// Field gain averaged over each microphone's pairs.
    pub mean_gains: &'a [Complex<f32>],
    /// Whole-field gain and timing fit per microphone.
    pub fits: &'a [Option<(Complex<f64>, Complex<f64>)>],
    pub refs: usize,
    pub bins: usize,
}

pub(super) struct ShortChange {
    window: usize,
    size: usize,
    segment: usize,
    /// Window index of the transform's first sample.
    origin: usize,
    /// Transform index of the segment's first sample.
    start: usize,
    pre: usize,
    taps: usize,
    taper: Vec<f32>,
    /// The whole window's energy over the taper's sum: converts the long-term reference
    /// spectrum into this segment's correlation scale.
    scale: f32,
    taper_energy: f32,
    window_energy: f32,
    forward: Arc<dyn Fft<f32>>,
    inverse: Arc<dyn Fft<f32>>,
    scratch: Vec<Complex<f32>>,
    references: Vec<Complex<f32>>,
    mean: Vec<Complex<f32>>,
    auto: Vec<Complex<f32>>,
    buffer: Vec<Complex<f32>>,
    change: Vec<Complex<f32>>,
    current: Vec<Complex<f32>>,
    smooth: usize,
    smoothed: Vec<f32>,
}
impl ShortChange {
    pub(super) fn new(config: &EngineConfig, refs: usize) -> Option<Self> {
        if !config.residual_motion || config.change_window == 0 {
            return None;
        }
        let (window, size, segment) = (config.window, config.change_size(), config.change_window);
        let mut planner = FftPlanner::new();
        let forward = planner.plan_fft_forward(size);
        let inverse = planner.plan_fft_inverse(size);
        let scratch = vec![
            Complex::default();
            forward
                .get_inplace_scratch_len()
                .max(inverse.get_inplace_scratch_len())
        ];
        let taper: Vec<f32> = (0..segment)
            .map(|i| 0.5 - 0.5 * (TAU * (i as f32 + 0.5) / segment as f32).cos())
            .collect();
        let window_energy = (0..window)
            .map(|i| (0.5 - 0.5 * (TAU * i as f32 / window as f32).cos()).powi(2))
            .sum::<f32>();
        let bins = size / 2 + 1;
        Some(Self {
            window,
            size,
            segment,
            origin: (window - size) / 2,
            start: (size - segment) / 2,
            pre: config.pre_taps,
            taps: config.pre_taps + config.taps,
            scale: window_energy / taper.iter().sum::<f32>(),
            taper_energy: taper.iter().map(|v| v * v).sum(),
            window_energy,
            taper,
            forward,
            inverse,
            scratch,
            references: vec![Complex::default(); size * refs],
            mean: vec![Complex::default(); size],
            auto: vec![Complex::default(); bins],
            buffer: vec![Complex::default(); size],
            change: vec![Complex::default(); bins],
            current: vec![Complex::default(); bins],
            smooth: config.change_smooth_bins,
            smoothed: vec![0.0; bins],
        })
    }
    /// Writes each microphone's change into the first pair slot of `dynamic`, and the current
    /// field (the background map plus this segment's own estimate) into `normalized`.
    pub(super) fn process(
        &mut self,
        echo: &EchoEstimator,
        inputs: ShortInputs<'_>,
        mean_maps: &[Complex<f32>],
        dynamic: &mut [Complex<f32>],
        normalized: &mut [Complex<f32>],
    ) {
        let ShortInputs {
            transfer,
            gains,
            mean_gains,
            fits,
            refs,
            bins,
        } = inputs;
        let (size, half) = (self.size, self.size / 2);
        let ratio = self.window / size;
        let channels = echo.channels();
        let fraction = echo.fractional_delay();
        // The same fractional reference delay as the whole-window spectra.
        let delay = |bin: usize| {
            Complex::from_polar(
                1.0,
                (-std::f64::consts::TAU * bin as f64 * fraction / size as f64) as f32,
            )
        };
        let inputs = echo.reference_channels();
        for s in 0..inputs {
            let signal = &echo.reference_signal(s)[self.origin..self.origin + size];
            let spectrum = &mut self.references[s * size..(s + 1) * size];
            for (value, sample) in spectrum.iter_mut().zip(signal) {
                *value = Complex::new(*sample, 0.0);
            }
            self.forward
                .process_with_scratch(spectrum, &mut self.scratch);
            for (bin, value) in spectrum.iter_mut().enumerate().take(half) {
                *value *= delay(bin);
            }
        }
        for bin in 0..size {
            self.mean[bin] = self.references[..inputs * size]
                .iter()
                .skip(bin)
                .step_by(size)
                .sum::<Complex<f32>>()
                / inputs as f32;
        }
        // The tapered segment of the mean reference sets this segment's own level and the
        // per-bin scale of an echo it contains.
        self.buffer.fill(Complex::default());
        for s in 0..inputs {
            let signal = &echo.reference_signal(s)[self.origin + self.start..];
            let segment = &mut self.buffer[self.start..self.start + self.segment];
            for ((value, sample), taper) in segment.iter_mut().zip(signal).zip(&self.taper) {
                value.re += sample * taper / inputs as f32;
            }
        }
        self.forward
            .process_with_scratch(&mut self.buffer, &mut self.scratch);
        let (mut own, mut average, mut auto_power, mut count) = (0.0_f64, 0.0_f64, 0.0_f64, 0);
        for bin in 1..half {
            let band = echo.band(bin * ratio);
            let tapered = self.buffer[bin] * delay(bin);
            self.auto[bin] = tapered * self.mean[bin].conj();
            if band == 0.0 {
                continue;
            }
            own += (band * tapered.norm_sqr()) as f64;
            average += (band * echo.long_term_mean_power(bin * ratio)) as f64;
            auto_power += self.auto[bin].norm_sqr() as f64;
            count += 1;
        }
        // This segment's own power, smoothed across frequency: follows a transient's spectral
        // shape without the noise of single bins.
        if self.smooth > 0 {
            for bin in 1..half {
                let first = bin.saturating_sub(self.smooth).max(1);
                let last = (bin + self.smooth).min(half - 1);
                self.smoothed[bin] = (first..=last).map(|b| self.auto[b].norm()).sum::<f32>()
                    / (last - first + 1) as f32;
            }
        }
        let level = if average > 1e-30 {
            (own / self.taper_energy as f64 / (average / self.window_energy as f64)) as f32
        } else {
            1.0
        };
        let regular = (1e-3 * average * level as f64 / count.max(1) as f64) as f32 + 1e-20;
        let auto_regular = (1e-3 * auto_power / count.max(1) as f64) as f32 + 1e-30;
        let length = self.taps * refs;
        dynamic.fill(Complex::default());
        normalized.fill(Complex::default());
        for c in 0..fits.len() {
            let first = c * refs;
            let gain = |pair: usize| {
                if gains[pair].norm_sqr() > 1e-20 {
                    gains[pair]
                } else {
                    Complex::new(1.0, 0.0)
                }
            };
            // The background's prediction of this microphone, as an analytic signal.
            self.buffer.fill(Complex::default());
            for bin in 1..half {
                let big = bin * ratio;
                if echo.band(big) == 0.0 {
                    continue;
                }
                let mut predicted = Complex::default();
                for s in 0..channels {
                    let reference = if channels == inputs {
                        self.references[s * size + bin]
                    } else {
                        self.mean[bin]
                    };
                    let pair = first + s;
                    predicted += gain(pair) * transfer[pair * bins + big] * reference;
                }
                if let Some((a, b)) = fits[c] {
                    let w = std::f64::consts::TAU * bin as f64 / size as f64;
                    let factor = Complex::new(1.0, 0.0) + a + b * Complex::new(0.0, w);
                    predicted *= Complex::new(factor.re as f32, factor.im as f32);
                }
                self.buffer[bin] = predicted * 2.0;
            }
            self.inverse
                .process_with_scratch(&mut self.buffer, &mut self.scratch);
            let microphone = &echo.microphone_signal(c)[self.origin + self.start..];
            for i in 0..size {
                self.buffer[i] = if (self.start..self.start + self.segment).contains(&i) {
                    let j = i - self.start;
                    let residual = microphone[j] - self.buffer[i].re / size as f32;
                    Complex::new(residual * self.taper[j], 0.0)
                } else {
                    Complex::default()
                };
            }
            self.forward
                .process_with_scratch(&mut self.buffer, &mut self.scratch);
            let mean_gain = if mean_gains[c].norm_sqr() > 1e-20 {
                mean_gains[c]
            } else {
                Complex::new(1.0, 0.0)
            };
            for bin in 0..half {
                self.change[bin] = Complex::default();
                self.current[bin] = Complex::default();
                let big = bin * ratio;
                if bin == 0 || echo.band(big) == 0.0 {
                    continue;
                }
                let cross = self.buffer[bin] * self.mean[bin].conj();
                self.change[bin] = if self.smooth > 0 {
                    cross / (mean_gain * (self.smoothed[bin] + auto_regular.sqrt()))
                } else {
                    cross * self.scale
                        / (mean_gain * (echo.long_term_mean_power(big) * level + regular))
                };
                // The vanished-echo guard needs this segment's own estimate, as in the whole
                // window: an echo that left cancels the background exactly.
                let auto = self.auto[bin];
                self.current[bin] =
                    cross * auto.conj() / (mean_gain * (auto.norm_sqr() + auto_regular));
            }
            let slot = first * self.taps;
            for (spectrum, output) in [
                (0, &mut dynamic[slot..slot + self.taps]),
                (1, &mut normalized[slot..slot + self.taps]),
            ] {
                self.buffer.fill(Complex::default());
                let source = if spectrum == 0 {
                    &self.change
                } else {
                    &self.current
                };
                for (bin, value) in source.iter().enumerate().take(half).skip(1) {
                    self.buffer[bin] = value * (2.0 * echo.weights()[bin * ratio]);
                }
                self.inverse
                    .process_with_scratch(&mut self.buffer, &mut self.scratch);
                for (i, tap) in output.iter_mut().enumerate() {
                    let k = i as i64 - self.pre as i64;
                    *tap = self.buffer[k.rem_euclid(size as i64) as usize] / size as f32;
                }
            }
            for (tap, background) in normalized[slot..slot + self.taps]
                .iter_mut()
                .zip(&mean_maps[c * length..c * length + self.taps])
            {
                *tap += *background;
            }
        }
    }
}
