use crate::config::EngineConfig;
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::{f32::consts::TAU, sync::Arc};

/// Converts positive-frequency transfer functions into window-corrected analytic taps.
pub struct MapTransform {
    n: usize,
    bins: usize,
    pre: usize,
    taps: usize,
    correction: Vec<f32>,
    inverse: Arc<dyn Fft<f32>>,
    buffer: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
}
impl MapTransform {
    pub fn new(config: &EngineConfig) -> Result<Self, &'static str> {
        config.validate()?;
        let n = config.window;
        let window: Vec<_> = (0..n)
            .map(|i| 0.5 - 0.5 * (TAU * i as f32 / n as f32).cos())
            .collect();
        let energy = window.iter().map(|v| v * v).sum::<f32>();
        let correction = (-(config.pre_taps as i64)..config.taps as i64)
            .map(|k| {
                let shift = k.unsigned_abs() as usize;
                window[..n - shift]
                    .iter()
                    .zip(&window[shift..])
                    .map(|(a, b)| a * b)
                    .sum::<f32>()
                    / energy
            })
            .collect();
        let inverse = FftPlanner::new().plan_fft_inverse(n);
        let scratch = vec![Complex::default(); inverse.get_inplace_scratch_len()];
        Ok(Self {
            n,
            bins: n / 2 + 1,
            pre: config.pre_taps,
            taps: config.pre_taps + config.taps,
            correction,
            inverse,
            buffer: vec![Complex::default(); n],
            scratch,
        })
    }
    /// Pairs are consecutive, each containing N/2+1 bins or P+K output taps.
    pub fn render(
        &mut self,
        transfer: &[Complex<f32>],
        weights: &[f32],
        output: &mut [Complex<f32>],
    ) -> Result<(), &'static str> {
        if weights.len() != self.bins
            || !transfer.len().is_multiple_of(self.bins)
            || output.len() != transfer.len() / self.bins * self.taps
        {
            return Err("Invalid echo-map dimensions.");
        }
        for (spectrum, taps) in transfer.chunks(self.bins).zip(output.chunks_mut(self.taps)) {
            // Mono mode leaves the second reference slot of each microphone empty.
            if spectrum.iter().all(|value| *value == Complex::default()) {
                taps.fill(Complex::default());
                continue;
            }
            self.buffer.fill(Complex::default());
            for bin in 1..self.n / 2 {
                self.buffer[bin] = spectrum[bin] * (2.0 * weights[bin]);
            }
            self.inverse
                .process_with_scratch(&mut self.buffer, &mut self.scratch);
            for (i, tap) in taps.iter_mut().enumerate() {
                let k = i as i64 - self.pre as i64;
                *tap = self.buffer[k.rem_euclid(self.n as i64) as usize]
                    / (self.n as f32 * self.correction[i]);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DirectPeak {
    pub position: f64,
    pub variance: f64,
    pub energy: f32,
}
/// Sum the measured analytic-map energy over every microphone/reference pair.
pub fn direct_peak(maps: &[Complex<f32>], pre: usize, taps: usize) -> DirectPeak {
    direct_peak_near(maps, pre, taps, None, 2.0, 0.5)
}
/// The earliest direct sound at least `peak_ratio` (in amplitude) as strong as the strongest:
/// the nearest speaker and microphone. Echo distances count from it, so a hand reads the same
/// distance whichever microphone's direct sound happens to be strongest.
pub(crate) fn nearest_direct(
    maps: &[Complex<f32>],
    pre: usize,
    taps: usize,
    peak_ratio: f32,
) -> Option<f64> {
    let length = pre.checked_add(taps).filter(|length| *length >= 3)?;
    if maps.is_empty() || !maps.len().is_multiple_of(length) || pre >= length - 1 {
        return None;
    }
    let at = |i: usize| {
        maps.chunks(length)
            .map(|pair| pair[i].norm_sqr())
            .sum::<f32>()
    };
    let last = pre.saturating_add(16).min(length - 2);
    let strongest = (0..=last).map(at).fold(0.0_f32, f32::max);
    if strongest <= 1e-20 {
        return None;
    }
    let index = (1..=last).find(|&index| {
        at(index) >= at(index - 1)
            && at(index) >= at(index + 1)
            && at(index) >= strongest * peak_ratio * peak_ratio
    })?;
    let (left, centre, right) = (at(index - 1), at(index), at(index + 1));
    let curvature = 2.0 * centre - left - right;
    let fraction = if curvature > 1e-20 {
        (0.5 * (right - left) / curvature).clamp(-0.5, 0.5)
    } else {
        0.0
    };
    Some(index as f64 - pre as f64 + fraction as f64)
}
pub(crate) fn direct_peak_near(
    maps: &[Complex<f32>],
    pre: usize,
    taps: usize,
    expected: Option<f64>,
    tolerance: f64,
    peak_ratio: f32,
) -> DirectPeak {
    let Some(length) = pre.checked_add(taps).filter(|length| *length >= 2) else {
        return DirectPeak::default();
    };
    if maps.is_empty() || !maps.len().is_multiple_of(length) || pre >= length - 1 {
        return DirectPeak::default();
    }
    let at = |i: usize| {
        maps.chunks(length)
            .map(|pair| pair[i].norm_sqr())
            .sum::<f32>()
    };
    let last = pre.saturating_add(16).min(length - 2);
    let global = (0..=last)
        .max_by(|a, b| at(*a).total_cmp(&at(*b)))
        .unwrap_or(pre);
    let index = expected
        .filter(|value| value.is_finite())
        .and_then(|expected| {
            let first = (expected + pre as f64 - tolerance).ceil().max(1.0) as usize;
            let end = (expected + pre as f64 + tolerance)
                .floor()
                .max(0.0)
                .min(last as f64) as usize;
            (first..=end)
                .filter(|&index| {
                    at(index) >= at(index - 1)
                        && at(index) >= at(index + 1)
                        && at(index) >= at(global) * peak_ratio * peak_ratio
                })
                .max_by(|a, b| at(*a).total_cmp(&at(*b)))
        })
        .unwrap_or(global);
    let centre = at(index);
    let left = at(index.saturating_sub(1));
    let right = at(index + 1);
    let curvature = 2.0 * centre - left - right;
    let fraction = if index != 0 && curvature > 1e-20 {
        (0.5 * (right - left) / curvature).clamp(-0.5, 0.5)
    } else {
        0.0
    };
    DirectPeak {
        position: index as f64 - pre as f64 + fraction as f64,
        variance: ((centre / curvature.max(1e-20)) as f64 * 0.01).clamp(0.001, 4.0),
        energy: centre,
    }
}
