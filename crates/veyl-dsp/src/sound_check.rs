//! An opt-in diagnostic measures high-band correlation and Welch coherence in memory.
use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, Default)]
pub struct Result {
    pub lag_frames: i32,
    pub mean_coherence: f32,
    pub coverage: f32,
    pub effective_bandwidth_hz: f32,
    pub score: f32,
}
pub fn analyze(
    reference: &[f32],
    microphone: &[f32],
    rate: u32,
) -> std::result::Result<Result, &'static str> {
    if ![44100, 48000].contains(&rate)
        || reference.len().max(microphone.len()) > rate as usize * 8
        || reference
            .iter()
            .chain(microphone)
            .any(|sample| !sample.is_finite())
    {
        return Err("Invalid sound-check input.");
    }
    let length = reference.len().min(microphone.len());
    if length < 3072 {
        return Ok(Result::default());
    }
    let mut planner = FftPlanner::<f32>::new();
    // Two seconds bound the alignment FFT; the full input contributes to Welch's averages.
    let align_length = length.min(rate as usize * 2);
    let size = (align_length * 2).next_power_of_two();
    let forward = planner.plan_fft_forward(size);
    let inverse = planner.plan_fft_inverse(size);
    let mut x = vec![Complex::default(); size];
    let mut y = x.clone();
    for i in 0..align_length {
        x[i].re = reference[i];
        y[i].re = microphone[i];
    }
    let mut scratch = vec![
        Complex::default();
        forward
            .get_inplace_scratch_len()
            .max(inverse.get_inplace_scratch_len())
    ];
    forward.process_with_scratch(&mut x, &mut scratch);
    forward.process_with_scratch(&mut y, &mut scratch);
    for i in 0..size {
        let hz = i.min(size - i) as f32 * rate as f32 / size as f32;
        x[i] = if (8000.0..=20000.0).contains(&hz) {
            y[i] * x[i].conj()
        } else {
            Complex::default()
        };
    }
    inverse.process_with_scratch(&mut x, &mut scratch);
    let limit = (rate as usize / 2).min(align_length - 3072) as i32;
    let lag = (-limit..=limit)
        .max_by(|a, b| {
            let value = |lag: i32| x[lag.rem_euclid(size as i32) as usize].re.abs();
            value(*a).total_cmp(&value(*b))
        })
        .unwrap_or(0);
    if x[lag.rem_euclid(size as i32) as usize].norm_sqr() < 1e-20 {
        return Ok(Result::default());
    }
    drop((x, y, scratch, forward, inverse));
    let ref_start = (-lag).max(0) as usize;
    let mic_start = lag.max(0) as usize;
    let length = (reference.len() - ref_start).min(microphone.len() - mic_start);
    const N: usize = 2048;
    let fft = planner.plan_fft_forward(N);
    let window: Vec<_> = (0..N)
        .map(|i| 0.5 - 0.5 * (TAU * i as f32 / (N - 1) as f32).cos())
        .collect();
    let mut x = vec![Complex::default(); N];
    let mut y = x.clone();
    let mut scratch = vec![Complex::default(); fft.get_inplace_scratch_len()];
    let mut xx = vec![0.0_f64; N / 2 + 1];
    let mut yy = xx.clone();
    let mut xy = vec![Complex::<f64>::default(); xx.len()];
    for offset in (0..=length - N).step_by(N / 2) {
        for i in 0..N {
            x[i] = Complex::new(reference[ref_start + offset + i] * window[i], 0.0);
            y[i] = Complex::new(microphone[mic_start + offset + i] * window[i], 0.0);
        }
        fft.process_with_scratch(&mut x, &mut scratch);
        fft.process_with_scratch(&mut y, &mut scratch);
        for bin in 0..xx.len() {
            xx[bin] += x[bin].norm_sqr() as f64;
            yy[bin] += y[bin].norm_sqr() as f64;
            let cross = y[bin] * x[bin].conj();
            xy[bin] += Complex::new(cross.re as f64, cross.im as f64);
        }
    }
    let first = (8000.0 * N as f64 / rate as f64).ceil() as usize;
    let last = (20000.0 * N as f64 / rate as f64).floor() as usize;
    let maximum = xx[first..=last].iter().copied().fold(0.0_f64, f64::max);
    let mut mean = 0.0;
    let mut covered = 0;
    let mut active = 0;
    for bin in first..=last {
        let power = xx[bin] * yy[bin];
        let coherence = if power > 1e-20 {
            (xy[bin].norm_sqr() / power).clamp(0.0, 1.0)
        } else {
            0.0
        };
        mean += coherence;
        if coherence >= 0.5 {
            covered += 1;
        }
        if maximum > 1e-20 && xx[bin] >= maximum * 0.001 {
            active += 1;
        }
    }
    let mean = (mean / (last - first + 1) as f64) as f32;
    let coverage = covered as f32 / (last - first + 1) as f32;
    Ok(Result {
        lag_frames: lag,
        mean_coherence: mean,
        coverage,
        effective_bandwidth_hz: active as f32 * rate as f32 / N as f32,
        score: mean * coverage,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn noise(size: usize, seed: u64) -> Vec<f32> {
        let mut state = seed;
        (0..size)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state >> 40) as f32 / 16777216.0 - 0.5
            })
            .collect()
    }
    #[test]
    fn alignment_and_coherence_find_positive_and_negative_delay_without_confusing_noise_or_tones() {
        for rate in [44100, 48000] {
            let reference = noise(rate as usize, 7);
            let mut microphone = vec![0.0; reference.len()];
            for i in 157..reference.len() {
                microphone[i] = reference[i - 157] * 0.4;
            }
            let result = analyze(&reference, &microphone, rate).unwrap();
            assert_eq!(result.lag_frames, 157);
            assert!(result.mean_coherence > 0.99);
            assert!(result.effective_bandwidth_hz > 11000.0);
            assert_eq!(
                analyze(&microphone, &reference, rate).unwrap().lag_frames,
                -157
            );
            let unrelated = analyze(&reference, &noise(reference.len(), 21), rate).unwrap();
            assert!(unrelated.mean_coherence < 0.1);
            assert!(unrelated.score < 0.1);
            let tone: Vec<_> = (0..reference.len())
                .map(|i| (TAU * 12000.0 * i as f32 / rate as f32).sin() * 0.1)
                .collect();
            assert!(analyze(&tone, &tone, rate).unwrap().effective_bandwidth_hz < 1000.0);
        }
        let silence = vec![0.0; 48000];
        assert_eq!(analyze(&silence, &silence, 48000).unwrap().score, 0.0);
        assert!(analyze(&[f32::NAN; 32], &silence, 48000).is_err());
        assert!(analyze(&silence, &silence, 16000).is_err());
    }
}
