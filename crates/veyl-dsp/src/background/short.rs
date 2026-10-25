//! Measure change over a short, central part of the window.
//!
//! A hand moving toward the microphone shortens its echo's delay during the window. Over the
//! whole 43 ms window, a 50 cm/s approach turns the high band's phase by most of a cycle, so the
//! correlation cancels and the moving hand disappears from the map. A segment a quarter as
//! long keeps that echo sharp. The background is still learned over the whole window.
use crate::{
    config::EngineConfig,
    echo::{gate::ImpulseGate, EchoEstimator},
};
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
    /// This hop's fractional-delay phase factors per bin.
    delays: Vec<Complex<f32>>,
    smooth: usize,
    smoothed: Vec<f32>,
    /// Running sums of the segment's own magnitudes, for smoothing.
    magnitudes: Vec<f64>,
    blank_ratio: f32,
    own_fit: bool,
    derivative: Vec<Complex<f32>>,
    /// The microphone region's spectrum, then the in-band residual's.
    /// Per microphone: the region's spectrum, then its in-band residual's, then the tapered
    /// segment residual's.
    regions: Vec<Complex<f32>>,
    packed: Vec<Complex<f32>>,
    in_band: bool,
    /// Per microphone, segment-major: the prediction, its quadrature and their time slopes.
    basis: Vec<[f32; 4]>,
    residual: Vec<f32>,
    clicks: Vec<bool>,
    kept: Vec<bool>,
    sums: Vec<[f64; 2]>,
    ratios: Vec<f32>,
    sorted: Vec<f32>,
    gate: Option<ImpulseGate>,
    /// Window positions of filled-in missing frames for the next hop, and running counts of the
    /// reference's.
    microphone_gaps: Vec<bool>,
    reference_gaps: Vec<u32>,
    gaps: bool,
}
#[derive(Clone, Copy)]
enum Target {
    References,
    Regions,
}
/// Half a millisecond either side at 48 kHz: the click envelope's smoothing.
const BLANK_HALF_WIDTH: usize = 24;
/// A click's decay and the privacy filter's ringing outlast its loudest part, so blanking
/// covers 4 ms after it and 0.5 ms before.
const BLANK_AFTER: usize = 192;
const BLANK_BEFORE: usize = 24;
impl ShortChange {
    pub(super) fn new(config: &EngineConfig, rate: u32, refs: usize, mics: usize) -> Option<Self> {
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
            origin: config.change_placement().0,
            start: config.change_placement().1,
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
            delays: vec![Complex::default(); bins],
            smooth: config.change_smooth_bins,
            smoothed: vec![0.0; bins],
            magnitudes: vec![0.0; bins],
            blank_ratio: config.change_blank_ratio,
            own_fit: config.change_own_fit,
            derivative: vec![Complex::default(); size],
            regions: vec![Complex::default(); size * mics],
            packed: vec![Complex::default(); size],
            in_band: config.change_in_band,
            basis: vec![[0.0; 4]; segment * mics],
            residual: vec![0.0; segment * mics],
            clicks: vec![false; segment],
            kept: vec![true; segment],
            sums: vec![[0.0; 2]; segment + 1],
            ratios: vec![0.0; segment],
            sorted: vec![0.0; segment],
            microphone_gaps: vec![false; window],
            reference_gaps: vec![0; window + 1],
            gaps: false,
            gate: (config.change_impulse_ratio > 0.0).then(|| {
                ImpulseGate::new(
                    (2 * rate as usize).div_ceil(config.hop),
                    config.change_impulse_ratio,
                    0,
                )
            }),
        })
    }
    /// Transforms one or two real signals, `size` samples each, with one complex FFT into
    /// `packed`: the first as the real part and the second as the imaginary part.
    fn transform_pair(&mut self, first: &[f32], second: Option<&[f32]>) {
        for (i, value) in self.packed.iter_mut().enumerate() {
            *value = Complex::new(first[i], second.map_or(0.0, |signal| signal[i]));
        }
        self.forward
            .process_with_scratch(&mut self.packed, &mut self.scratch);
    }
    /// Separates `packed` into the two real signals' spectra by conjugate symmetry, writing
    /// bins 0 to N/2 at the given offsets.
    fn split_pair(&mut self, first: usize, second: Option<usize>, target: Target) {
        let size = self.size;
        let output = match target {
            Target::References => &mut self.references,
            Target::Regions => &mut self.regions,
        };
        for bin in 0..=size / 2 {
            let (z, mirror) = (self.packed[bin], self.packed[(size - bin) % size].conj());
            output[first + bin] = (z + mirror) * 0.5;
            if let Some(offset) = second {
                output[offset + bin] = (z - mirror) * Complex::new(0.0, -0.5);
            }
        }
    }
    /// Marks where a click or knock dominates one microphone's residual. Such a sound is far
    /// louder than the residual relative to the predicted music, and lasts a few milliseconds;
    /// a louder moment of the music raises both envelopes together, and a hand's echo is
    /// continuous.
    fn find_clicks(&mut self, c: usize) {
        let (length, half) = (self.segment, BLANK_HALF_WIDTH);
        let offset = c * length;
        self.sums[0] = [0.0; 2];
        for j in 0..length {
            self.sums[j + 1] = [
                self.sums[j][0] + self.residual[offset + j].abs() as f64,
                self.sums[j][1] + self.basis[offset + j][0].abs() as f64,
            ];
        }
        let mean = self.sums[length][1] / length as f64;
        for j in 0..length {
            let (first, last) = (j.saturating_sub(half), (j + half + 1).min(length));
            let residual = self.sums[last][0] - self.sums[first][0];
            let predicted = self.sums[last][1] - self.sums[first][1];
            self.ratios[j] =
                (residual / (predicted + 1e-3 * mean * (last - first) as f64 + 1e-12)) as f32;
        }
        self.sorted.copy_from_slice(&self.ratios);
        let middle = length / 2;
        let (_, median, _) = self.sorted.select_nth_unstable_by(middle, f32::total_cmp);
        let threshold = *median * self.blank_ratio;
        for (click, ratio) in self.clicks.iter_mut().zip(&self.ratios) {
            *click |= *ratio > threshold;
        }
    }
    pub(super) fn set_gaps(&mut self, microphone: &[bool], reference: &[bool]) {
        self.microphone_gaps.copy_from_slice(microphone);
        self.reference_gaps[0] = 0;
        for (index, missing) in reference.iter().enumerate() {
            self.reference_gaps[index + 1] = self.reference_gaps[index] + u32::from(*missing);
        }
        self.gaps = true;
    }
    /// A click reaches every microphone, so its span is blanked on all of them. Samples the
    /// microphone lost, or whose prediction needs lost reference, are blanked as well.
    fn blank_clicks(&mut self) {
        let length = self.segment;
        self.kept.fill(true);
        if std::mem::take(&mut self.gaps) {
            let first = self.origin + self.start;
            for j in 0..length {
                let t = first + j;
                // The prediction at t reads the reference from t - (P + K) to t + P.
                let low = t.saturating_sub(self.taps);
                let high = (t + self.pre + 1).min(self.window);
                if self.microphone_gaps[t] || self.reference_gaps[high] > self.reference_gaps[low] {
                    self.kept[j] = false;
                }
            }
        }
        let mut until = None;
        for j in 0..length {
            if self.clicks[j] {
                until = Some(j + BLANK_AFTER);
            }
            if until.is_some_and(|last| j <= last) {
                self.kept[j] = false;
            }
        }
        let mut from = None;
        for j in (0..length).rev() {
            if self.clicks[j] {
                from = Some(j.saturating_sub(BLANK_BEFORE));
            }
            if from.is_some_and(|first| j >= first) {
                self.kept[j] = false;
            }
        }
        for residual in self.residual.chunks_mut(length) {
            for (value, kept) in residual.iter_mut().zip(&self.kept) {
                if !kept {
                    *value = 0.0;
                }
            }
        }
    }
    /// Removes the gain, phase and sub-sample timing change of the whole predicted field over
    /// this segment, as the whole window does in the frequency domain. Fitting the segment
    /// itself, after blanking, keeps a click elsewhere in the window out of the fit.
    fn fit_field(&mut self, c: usize) {
        let offset = c * self.segment;
        let mut normal = [[0.0_f64; 4]; 4];
        let mut target = [0.0_f64; 4];
        for j in 0..self.segment {
            if !self.kept[j] {
                continue;
            }
            let weight = (self.taper[j] * self.taper[j]) as f64;
            let basis = self.basis[offset + j];
            for a in 0..4 {
                target[a] += weight * basis[a] as f64 * self.residual[offset + j] as f64;
                for b in 0..4 {
                    normal[a][b] += weight * basis[a] as f64 * basis[b] as f64;
                }
            }
        }
        let trace = (0..4).map(|a| normal[a][a]).sum::<f64>();
        if trace <= 1e-30 {
            return;
        }
        for (a, row) in normal.iter_mut().enumerate() {
            row[a] += 1e-9 * trace;
        }
        let Some(coefficients) = solve4(normal, target) else {
            return;
        };
        for j in 0..self.segment {
            if self.kept[j] {
                let basis = self.basis[offset + j];
                let fitted: f64 = (0..4).map(|a| coefficients[a] * basis[a] as f64).sum();
                self.residual[offset + j] -= fitted as f32;
            }
        }
    }
    /// Writes each microphone's change into the first pair slot of `dynamic`, and the current
    /// field (the background map plus this segment's own estimate) into `normalized`. Returns
    /// whether the segment holds an impulse that blanking could not remove.
    pub(super) fn process(
        &mut self,
        echo: &EchoEstimator,
        inputs: ShortInputs<'_>,
        mean_maps: &[Complex<f32>],
        dynamic: &mut [Complex<f32>],
        normalized: &mut [Complex<f32>],
        gate_enabled: bool,
    ) -> bool {
        let mut relative = 0.0_f64;
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
        crate::echo::phase_ramp(fraction, size, &mut self.delays);
        let inputs = echo.reference_channels();
        for first in (0..inputs).step_by(2) {
            let second = (first + 1 < inputs).then(|| first + 1);
            let origin = self.origin;
            let region = |channel: usize| &echo.reference_signal(channel)[origin..];
            self.transform_pair(region(first), second.map(region));
            self.split_pair(
                first * size,
                second.map(|channel| channel * size),
                Target::References,
            );
        }
        for bin in 0..half {
            let mut sum = Complex::default();
            for channel in 0..inputs {
                let value = &mut self.references[channel * size + bin];
                *value *= self.delays[bin];
                sum += *value;
            }
            self.mean[bin] = sum / inputs as f32;
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
            let tapered = self.buffer[bin] * self.delays[bin];
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
            // A running sum of magnitudes, each computed once.
            self.magnitudes[0] = 0.0;
            for bin in 1..half {
                self.magnitudes[bin] = self.magnitudes[bin - 1] + self.auto[bin].norm() as f64;
            }
            for bin in 1..half {
                let first = bin.saturating_sub(self.smooth).max(1);
                let last = (bin + self.smooth).min(half - 1);
                self.smoothed[bin] = ((self.magnitudes[last] - self.magnitudes[first - 1])
                    / (last - first + 1) as f64) as f32;
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
        let mics = fits.len();
        self.clicks.fill(false);
        self.kept.fill(true);
        if self.in_band {
            for first in (0..mics).step_by(2) {
                let second = (first + 1 < mics).then(|| first + 1);
                let origin = self.origin;
                let region = |channel: usize| &echo.microphone_signal(channel)[origin..];
                self.transform_pair(region(first), second.map(region));
                self.split_pair(first * size, second.map(|c| c * size), Target::Regions);
            }
        }
        for (c, fit) in fits.iter().enumerate() {
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
            let region = c * size;
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
                let w = std::f64::consts::TAU * bin as f64 / size as f64;
                if self.own_fit {
                    self.derivative[bin] = predicted * Complex::new(0.0, 2.0 * w as f32);
                } else if let Some((a, b)) = *fit {
                    let factor = Complex::new(1.0, 0.0) + a + b * Complex::new(0.0, w);
                    predicted *= Complex::new(factor.re as f32, factor.im as f32);
                }
                self.buffer[bin] = predicted * 2.0;
                if self.in_band {
                    // Only the band is predicted; outside it the microphone's own sound would
                    // count as residual and hide a click's rise.
                    let value = &mut self.regions[region + bin];
                    *value = (*value - predicted) * echo.band(big);
                }
            }
            if self.in_band {
                // Bins the band leaves out carry no residual.
                self.regions[region] = Complex::default();
                for bin in 1..half {
                    if echo.band(bin * ratio) == 0.0 {
                        self.regions[region + bin] = Complex::default();
                    }
                }
            }
            self.inverse
                .process_with_scratch(&mut self.buffer, &mut self.scratch);
            if self.own_fit {
                self.inverse
                    .process_with_scratch(&mut self.derivative, &mut self.scratch);
            }
            let microphone = &echo.microphone_signal(c)[self.origin + self.start..];
            let offset = c * self.segment;
            for (j, sample) in microphone.iter().enumerate().take(self.segment) {
                let (value, slope) = (self.buffer[self.start + j], self.derivative[self.start + j]);
                let basis = [value.re, -value.im, slope.re, -slope.im].map(|v| v / size as f32);
                self.basis[offset + j] = basis;
                if !self.in_band {
                    self.residual[offset + j] = sample - basis[0];
                }
            }
        }
        if self.in_band {
            // Two microphones' real residuals share one inverse transform.
            for first in (0..mics).step_by(2) {
                let second = (first + 1 < mics).then(|| first + 1);
                self.packed.fill(Complex::default());
                for bin in 1..half {
                    let a = self.regions[first * size + bin];
                    let b = second.map_or(Complex::default(), |c| self.regions[c * size + bin]);
                    self.packed[bin] = a + Complex::new(-b.im, b.re);
                    self.packed[size - bin] = a.conj() + Complex::new(b.im, b.re);
                }
                self.inverse
                    .process_with_scratch(&mut self.packed, &mut self.scratch);
                for j in 0..self.segment {
                    let value = self.packed[self.start + j] / size as f32;
                    self.residual[first * self.segment + j] = value.re;
                    if let Some(c) = second {
                        self.residual[c * self.segment + j] = value.im;
                    }
                }
            }
        }
        if self.blank_ratio > 0.0 {
            for c in 0..mics {
                self.find_clicks(c);
            }
        }
        if self.blank_ratio > 0.0 || self.gaps {
            self.blank_clicks();
        }
        let usable = self.kept.iter().filter(|kept| **kept).count();
        for c in 0..mics {
            if self.own_fit {
                self.fit_field(c);
            }
        }
        for first in (0..mics).step_by(2) {
            let second = (first + 1 < mics).then(|| first + 1);
            self.packed.fill(Complex::default());
            for j in 0..self.segment {
                let b = second.map_or(0.0, |c| self.residual[c * self.segment + j]);
                self.packed[self.start + j] = Complex::new(
                    self.residual[first * self.segment + j] * self.taper[j],
                    b * self.taper[j],
                );
            }
            self.forward
                .process_with_scratch(&mut self.packed, &mut self.scratch);
            self.split_pair(first * size, second.map(|c| c * size), Target::Regions);
        }
        for c in 0..mics {
            let first = c * refs;
            let offset = c * self.segment;
            let (mut kept, mut predicted) = (0.0_f64, 0.0_f64);
            for j in 0..self.segment {
                if self.kept[j] {
                    kept += (self.residual[offset + j] as f64).powi(2);
                    predicted += (self.basis[offset + j][0] as f64).powi(2);
                }
            }
            relative = relative.max(kept / predicted.max(1e-30));
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
                let cross = self.regions[c * size + bin] * self.mean[bin].conj();
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
        // Too little of the segment remains for a measurement.
        let unusable = usable * 2 < self.segment;
        self.gate
            .as_mut()
            .is_some_and(|gate| gate.observe(relative, gate_enabled))
            || unusable
    }
}

/// Gaussian elimination with partial pivoting for the small normal equations.
fn solve4(mut matrix: [[f64; 4]; 4], mut vector: [f64; 4]) -> Option<[f64; 4]> {
    for column in 0..4 {
        let pivot = (column..4).max_by(|a, b| {
            matrix[*a][column]
                .abs()
                .total_cmp(&matrix[*b][column].abs())
        })?;
        if matrix[pivot][column].abs() < 1e-300 {
            return None;
        }
        matrix.swap(column, pivot);
        vector.swap(column, pivot);
        for row in column + 1..4 {
            let factor = matrix[row][column] / matrix[column][column];
            let pivot_row = matrix[column];
            for (value, pivot) in matrix[row][column..].iter_mut().zip(&pivot_row[column..]) {
                *value -= factor * pivot;
            }
            vector[row] -= factor * vector[column];
        }
    }
    let mut result = [0.0; 4];
    for row in (0..4).rev() {
        let sum: f64 = (row + 1..4).map(|k| matrix[row][k] * result[k]).sum();
        result[row] = (vector[row] - sum) / matrix[row][row];
    }
    result.iter().all(|v| v.is_finite()).then_some(result)
}
