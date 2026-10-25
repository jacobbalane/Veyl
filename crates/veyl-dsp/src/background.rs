//! Normalize the direct sound before learning stationary echoes.
use crate::{
    config::EngineConfig,
    echo::{direct_peak_near, DirectPeak, EchoEstimator, MapTransform},
    StreamFormat,
};
use rustfft::num_complex::Complex;
use short::{ShortChange, ShortInputs};

mod short;
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
    mean_maps: Vec<Complex<f32>>,
    matched: Vec<Complex<f32>>,
    gains: Vec<Complex<f32>>,
    previous_gains: Vec<Complex<f32>>,
    transform: MapTransform,
    initialized: bool,
    previous_direct: Option<f64>,
    settle_frames: u64,
    presence_frames: u64,
    residual: Residual,
    short: Option<ShortChange>,
}
/// Buffers for measuring change from the microphone residual; see `Background::residual`.
struct Residual {
    refs: usize,
    mics: usize,
    alpha: f32,
    settle_alpha: f32,
    cross: Vec<Complex<f32>>,
    fast_power: Vec<f32>,
    change: Vec<Complex<f32>>,
    current: Vec<Complex<f32>>,
    mean: Vec<Complex<f32>>,
    learn: Vec<Complex<f32>>,
    predicted: Vec<Complex<f32>>,
    errors: Vec<Complex<f32>>,
    delays: Vec<Option<f64>>,
    hops: u32,
    /// Per microphone, for short change measurements.
    fits: Vec<Option<(Complex<f64>, Complex<f64>)>>,
    mean_gains: Vec<Complex<f32>>,
}
impl Residual {
    fn clear(&mut self) {
        self.cross.fill(Complex::default());
        self.fast_power.fill(0.0);
        self.change.fill(Complex::default());
        self.current.fill(Complex::default());
        self.mean.fill(Complex::default());
        self.learn.fill(Complex::default());
        self.delays.fill(None);
        self.fits.fill(None);
        self.mean_gains.fill(Complex::new(1.0, 0.0));
        self.hops = 0;
    }
}
impl Background {
    pub fn new(config: EngineConfig, format: StreamFormat) -> Result<Self, &'static str> {
        config.validate()?;
        format.validate()?;
        let bins = config.window / 2 + 1;
        let taps = config.pre_taps + config.taps;
        let pairs = format.ref_channels as usize * format.mic_channels as usize;
        let alpha =
            |time: f32| 1.0 - (-(config.hop as f32) / (time * format.sample_rate as f32)).exp();
        let residual = Residual {
            refs: format.ref_channels as usize,
            mics: format.mic_channels as usize,
            alpha: alpha(config.motion_seconds),
            settle_alpha: alpha(config.residual_settle_seconds),
            cross: vec![Complex::default(); bins * format.mic_channels as usize],
            fast_power: vec![0.0; bins],
            change: vec![Complex::default(); bins * pairs],
            current: vec![Complex::default(); bins * pairs],
            mean: vec![Complex::default(); bins * pairs],
            learn: vec![Complex::default(); bins * pairs],
            predicted: vec![Complex::default(); bins],
            errors: vec![Complex::default(); bins],
            delays: vec![None; format.mic_channels as usize],
            hops: 0,
            fits: vec![None; format.mic_channels as usize],
            mean_gains: vec![Complex::new(1.0, 0.0); format.mic_channels as usize],
        };
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
            mean_maps: vec![Complex::default(); taps * pairs],
            matched: vec![Complex::default(); bins * pairs],
            gains: vec![Complex::new(1.0, 0.0); pairs],
            previous_gains: vec![Complex::new(1.0, 0.0); pairs],
            transform: MapTransform::new(&config)?,
            initialized: false,
            previous_direct: None,
            settle_frames: (config.settle_seconds * format.sample_rate as f32).ceil() as u64,
            presence_frames: 0,
            residual,
            short: ShortChange::new(&config, format.ref_channels as usize),
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
    /// Residual mode: the background seen through the mean reference, in the first slot of each
    /// microphone.
    pub fn mean_maps(&self) -> &[Complex<f32>] {
        &self.mean_maps
    }
    /// Per microphone, the sub-sample delay of the whole field against the background in this
    /// hop, from residual mode.
    pub fn field_delays(&self) -> &[Option<f64>] {
        &self.residual.delays
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
        self.mean_maps.fill(Complex::default());
        self.matched.fill(Complex::default());
        self.gains.fill(Complex::new(1.0, 0.0));
        self.previous_gains.fill(Complex::new(1.0, 0.0));
        self.initialized = false;
        self.previous_direct = None;
        self.presence_frames = 0;
        self.residual.clear();
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
        self.process_with(transfer, maps, weights, hand_present, held, None)
    }
    /// Measures change from the microphone residual instead of the difference of two transfer
    /// estimates. The fast transfer estimate still sets the direct-path gain and model jumps.
    pub fn process_residual(
        &mut self,
        echo: &EchoEstimator,
        weights: &[f32],
        hand_present: bool,
        held: bool,
    ) -> Result<BackgroundFrame, &'static str> {
        self.process_with(
            echo.transfer(),
            echo.maps(),
            weights,
            hand_present,
            held,
            Some(echo),
        )
    }
    fn process_with(
        &mut self,
        transfer: &[Complex<f32>],
        maps: &[Complex<f32>],
        weights: &[f32],
        hand_present: bool,
        held: bool,
        echo: Option<&EchoEstimator>,
    ) -> Result<BackgroundFrame, &'static str> {
        if transfer.len() != self.transfer.len()
            || maps.len() != self.maps.len()
            || weights.len() != self.bins
        {
            return Err("Invalid background dimensions.");
        }
        let settling = !self.settled();
        let initial = !self.initialized;
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
            let (gain, energy) = match echo {
                // Settling learns the current scale; dividing its steps by a gain measured
                // against a half-learned field would feed back.
                Some(_) if settling => (Complex::new(1.0, 0.0), 0.0),
                // Correlated Stereo references leave each path's share ambiguous, so residual
                // mode compares the microphone with the whole predicted field instead.
                Some(echo) => self.field_gain(echo, pair / self.residual.refs),
                None if energy > 1e-20 => (cross / energy, energy),
                None => (Complex::new(1.0, 0.0), energy),
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
            if echo.is_none() {
                for k in 0..self.taps {
                    let i = pair * self.taps + k;
                    self.normalized_maps[i] = maps[i] / gain;
                    self.dynamic[i] = self.normalized_maps[i] - self.maps[i];
                }
            }
        }
        if let Some(echo) = echo {
            self.residual(echo);
            self.transform
                .render(&self.residual.mean, weights, &mut self.mean_maps)?;
            if let Some(short) = self.short.as_mut() {
                short.process(
                    echo,
                    ShortInputs {
                        transfer: &self.transfer,
                        gains: &self.gains,
                        mean_gains: &self.residual.mean_gains,
                        fits: &self.residual.fits,
                        refs: self.residual.refs,
                        bins: self.bins,
                    },
                    &self.mean_maps,
                    &mut self.dynamic,
                    &mut self.normalized_maps,
                );
            } else {
                self.transform
                    .render(&self.residual.change, weights, &mut self.dynamic)?;
                self.transform.render(
                    &self.residual.current,
                    weights,
                    &mut self.normalized_maps,
                )?;
            }
        }
        self.previous_gains.copy_from_slice(&self.gains);
        self.presence_frames = if hand_present {
            self.presence_frames.saturating_add(self.config.hop as u64)
        } else {
            0
        };
        if settling && echo.is_some() && !initial {
            // A fast normalized step converges without copying each noisy Stereo estimate.
            for (background, change) in self.transfer.iter_mut().zip(&self.residual.learn) {
                *background += self.residual.settle_alpha * *change;
            }
            self.settle_frames = self.settle_frames.saturating_sub(self.config.hop as u64);
        } else if settling {
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
            if echo.is_some() {
                for (background, change) in self.transfer.iter_mut().zip(&self.residual.learn) {
                    *background += beta * *change;
                }
            } else {
                for (background, current) in self.transfer.iter_mut().zip(&self.normalized) {
                    *background += beta * (*current - *background);
                }
            }
        }
        if echo.is_some() {
            for (pair, (matched, background)) in self
                .matched
                .chunks_mut(self.bins)
                .zip(self.transfer.chunks(self.bins))
                .enumerate()
            {
                for (matched, background) in matched.iter_mut().zip(background) {
                    *matched = *background * self.gains[pair];
                }
            }
        }
        Ok(result(false))
    }
    /// Residual mode keeps the background in normalized units and a separate field gain per
    /// microphone. Predictions of the microphone, such as the impulse gate's, need both.
    pub fn gain_matched_transfer(&self) -> &[Complex<f32>] {
        &self.matched
    }
    /// The complex gain that best maps the background prediction onto this microphone, with the
    /// predicted band energy.
    fn field_gain(&self, echo: &EchoEstimator, mic: usize) -> (Complex<f32>, f32) {
        let mut cross = Complex::<f64>::default();
        let mut energy = 0.0_f64;
        for bin in 0..self.bins {
            let band = echo.band(bin) as f64;
            if band == 0.0 {
                continue;
            }
            let mut predicted = Complex::<f32>::default();
            for s in 0..echo.channels() {
                let pair = mic * self.residual.refs + s;
                predicted += self.transfer[pair * self.bins + bin] * echo.reference(s, bin);
            }
            let microphone = echo.microphone(mic, bin);
            let p = Complex::new(predicted.re as f64, predicted.im as f64);
            cross += band * Complex::new(microphone.re as f64, microphone.im as f64) * p.conj();
            energy += band * p.norm_sqr();
        }
        if energy > 1e-30 {
            let gain = cross / energy;
            (Complex::new(gain.re as f32, gain.im as f32), energy as f32)
        } else {
            (Complex::new(1.0, 0.0), 0.0)
        }
    }
    /// Compares the microphone with the gain-matched background prediction, then matched-filters
    /// that residual against the mean reference, whitened by its long-term spectrum.
    ///
    /// A hand reflects every speaker, so one projection keeps most of its echo without a per-hop
    /// matrix inverse; correlated Stereo references made that inverse noisy. The background
    /// still predicts each speaker separately and learns each path with a normalized LMS step.
    fn residual(&mut self, echo: &EchoEstimator) {
        let channels = echo.channels();
        let state = &mut self.residual;
        let (bins, refs) = (self.bins, state.refs);
        let unit = Complex::new(1.0, 0.0);
        let mut current = 0.0;
        let mut average = 0.0;
        let mut count = 0;
        for bin in 0..bins {
            let band = echo.band(bin);
            if band == 0.0 {
                continue;
            }
            let power = &mut state.fast_power[bin];
            *power += state.alpha * (echo.mean_reference(bin).norm_sqr() - *power);
            current += (band * *power) as f64;
            average += (band * echo.long_term_mean_power(bin)) as f64;
            count += 1;
        }
        // One fast level scales the long-term spectra, so loud hops are not overcounted.
        let level = if average > 1e-30 {
            (current / average) as f32
        } else {
            1.0
        };
        let regular = (1e-3 * current / count.max(1) as f64) as f32 + 1e-20;
        state.hops = state.hops.saturating_add(1);
        for c in 0..state.mics {
            let first = c * refs;
            let gain = |pair: usize| {
                if self.gains[pair].norm_sqr() > 1e-20 {
                    self.gains[pair]
                } else {
                    unit
                }
            };
            let mut mean_gain = Complex::default();
            for s in 0..channels {
                mean_gain += gain(first + s);
            }
            mean_gain /= channels as f32;
            state.mean_gains[c] = mean_gain;
            for bin in 0..bins {
                state.predicted[bin] = Complex::default();
                state.errors[bin] = Complex::default();
                if echo.band(bin) == 0.0 {
                    continue;
                }
                for s in 0..channels {
                    let pair = first + s;
                    state.predicted[bin] +=
                        gain(pair) * self.transfer[pair * bins + bin] * echo.reference(s, bin);
                }
                state.errors[bin] = echo.microphone(c, bin) - state.predicted[bin];
            }
            let fit = whole_field_fit(&state.predicted, &state.errors, |bin| echo.band(bin));
            state.fits[c] = fit;
            // errors ~ -j w delay (1 + a) P for a field arriving `delay` frames late. A
            // background that has just started cannot time the field yet.
            state.delays[c] = fit
                .filter(|_| state.hops >= 16)
                .map(|(a, b)| -(b / (Complex::new(1.0, 0.0) + a)).re)
                .filter(|delay| delay.is_finite() && delay.abs() < 1.0);
            for bin in 0..bins {
                let motion = first * bins + bin;
                for s in 0..refs {
                    let i = (first + s) * bins + bin;
                    state.change[i] = Complex::default();
                    state.current[i] = Complex::default();
                    state.mean[i] = Complex::default();
                    state.learn[i] = Complex::default();
                }
                if echo.band(bin) == 0.0 {
                    continue;
                }
                let predicted = state.predicted[bin];
                let errors = state.errors[bin];
                // Small gain and timing changes of the whole field are not new echoes.
                let moving = fit.map_or(errors, |(a, b)| {
                    let w = std::f64::consts::TAU * bin as f64 / (2 * (bins - 1)) as f64;
                    let p = Complex::new(predicted.re as f64, predicted.im as f64);
                    let fitted = a * p + b * p * Complex::new(0.0, w);
                    errors - Complex::new(fitted.re as f32, fitted.im as f32)
                });
                let x = echo.mean_reference(bin);
                let cross = &mut state.cross[c * bins + bin];
                *cross += state.alpha * (moving * x.conj() - *cross);
                let mean_power = echo.long_term_mean_power(bin);
                state.change[motion] = *cross / (mean_gain * (mean_power * level + regular));
                // The guard against vanished echoes needs an exact per-window estimate, so this
                // copy uses the fast power of this window rather than the long-term spectrum.
                let mut mean_background = Complex::default();
                for s in 0..channels {
                    let pair = first + s;
                    let i = pair * bins + bin;
                    let reference = echo.reference(s, bin);
                    let (power, share) = if channels == 1 {
                        (mean_power, unit * mean_power)
                    } else if s == 0 {
                        let power = echo.long_term_power(0, bin);
                        (power, (echo.long_term_cross(bin) + power) * 0.5)
                    } else {
                        let power = echo.long_term_power(1, bin);
                        (power, (echo.long_term_cross(bin).conj() + power) * 0.5)
                    };
                    mean_background += self.transfer[i] * share;
                    // The instantaneous power bounds each step, as in normalized LMS.
                    let bound = (power * level).max(reference.norm_sqr()) + regular;
                    state.learn[i] = errors * reference.conj() / (gain(pair) * bound);
                }
                if channels == 2 {
                    // Correlated channels excite their difference weakly. The long-term
                    // covariance inverse learns that direction faster than separate steps;
                    // diagonal loading bounds the step while that covariance is still young.
                    let widen = |v: Complex<f32>| Complex::new(v.re as f64, v.im as f64);
                    let narrow = |v: Complex<f64>| Complex::new(v.re as f32, v.im as f32);
                    let (left, right) = (echo.reference(0, bin), echo.reference(1, bin));
                    let floor = regular as f64;
                    let a = ((echo.long_term_power(0, bin) * level).max(left.norm_sqr())) as f64;
                    let d = ((echo.long_term_power(1, bin) * level).max(right.norm_sqr())) as f64;
                    let loading = 0.1 * (a + d) * 0.5 + floor;
                    let (a, d) = (a + loading, d + loading);
                    let b = widen(echo.long_term_cross(bin)) * level as f64;
                    let determinant = (a * d - b.norm_sqr()).max(loading * loading);
                    let (u, v) = (widen(errors * left.conj()), widen(errors * right.conj()));
                    state.learn[first * bins + bin] =
                        narrow((u * d - v * b.conj()) / determinant) / gain(first);
                    state.learn[(first + 1) * bins + bin] =
                        narrow((v * a - u * b) / determinant) / gain(first + 1);
                }
                state.mean[motion] = mean_background / (mean_power + regular / level.max(1e-6));
                state.current[motion] =
                    state.mean[motion] + *cross / (mean_gain * (state.fast_power[bin] + regular));
            }
        }
    }
}
/// Least squares over the band: errors ~ a * P + b * (j w P). The pair spans small gain and
/// sub-sample timing changes of the whole predicted field, not a new echo at another delay.
fn whole_field_fit(
    predicted: &[Complex<f32>],
    errors: &[Complex<f32>],
    band: impl Fn(usize) -> f32,
) -> Option<(Complex<f64>, Complex<f64>)> {
    let bins = predicted.len();
    let (mut pp, mut qq) = (0.0_f64, 0.0_f64);
    let (mut pq, mut pe, mut qe) = (
        Complex::<f64>::default(),
        Complex::<f64>::default(),
        Complex::<f64>::default(),
    );
    for bin in 0..bins {
        let weight = band(bin) as f64;
        if weight == 0.0 {
            continue;
        }
        let w = std::f64::consts::TAU * bin as f64 / (2 * (bins - 1)) as f64;
        let p = Complex::new(predicted[bin].re as f64, predicted[bin].im as f64);
        let q = p * Complex::new(0.0, w);
        let e = Complex::new(errors[bin].re as f64, errors[bin].im as f64);
        pp += weight * p.norm_sqr();
        qq += weight * q.norm_sqr();
        pq += weight * p.conj() * q;
        pe += weight * p.conj() * e;
        qe += weight * q.conj() * e;
    }
    let determinant = pp * qq - pq.norm_sqr();
    (pp > 1e-30 && determinant > 1e-12 * pp * qq).then(|| {
        (
            (pe * qq - pq * qe) / determinant,
            (qe * pp - pq.conj() * pe) / determinant,
        )
    })
}

#[cfg(test)]
mod tests;
