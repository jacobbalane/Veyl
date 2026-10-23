//! Fractional sampling with a phase-interpolated Kaiser-windowed sinc.
pub struct FractionalSinc<const TAPS: usize> {
    weights: Vec<[f32; TAPS]>,
}
fn i0(value: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    for order in 1..50 {
        term *= value * value / (4.0 * (order * order) as f64);
        sum += term;
        if term < sum * 1e-16 {
            break;
        }
    }
    sum
}
impl<const TAPS: usize> FractionalSinc<TAPS> {
    pub fn new() -> Result<Self, &'static str> {
        if !(8..=128).contains(&TAPS) || !TAPS.is_multiple_of(2) {
            return Err("Fractional sampling needs an even tap count from 8 to 128.");
        }
        let half = TAPS / 2;
        let left = half.saturating_sub(1);
        let denominator = i0(8.6);
        let weights = (0..=1024)
            .map(|phase| {
                let mut weights = [0.0; TAPS];
                if phase == 0 {
                    weights[left] = 1.0;
                } else if phase == 1024 {
                    weights[left + 1] = 1.0;
                } else {
                    let fraction = phase as f64 / 1024.0;
                    let raw = std::array::from_fn::<_, TAPS, _>(|i| {
                        let x = fraction - (i as f64 - left as f64);
                        if x.abs() >= half as f64 {
                            0.0
                        } else {
                            let sinc = if x.abs() < 1e-12 {
                                1.0
                            } else {
                                (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x)
                            };
                            sinc * i0(8.6 * (1.0 - (x / half as f64).powi(2)).sqrt()) / denominator
                        }
                    });
                    let sum = raw.iter().sum::<f64>();
                    for (weight, value) in weights.iter_mut().zip(raw) {
                        *weight = (value / sum) as f32;
                    }
                }
                weights
            })
            .collect();
        Ok(Self { weights })
    }
    #[inline]
    pub fn read(&self, input: &[f32], at: f64) -> f32 {
        if at.is_finite()
            && at >= 0.0
            && at < input.len() as f64
            && input.len() <= isize::MAX as usize / 4
        {
            let base = at.floor() as usize;
            let left = TAPS / 2 - 1;
            if let Some(first) = base.checked_sub(left) {
                if let Some(samples) = input.get(first..first + TAPS) {
                    let phase = (at - base as f64) * 1024.0;
                    let lower = (phase.floor() as usize).min(1023);
                    let mix = (phase - lower as f64) as f32;
                    let low = &self.weights[lower];
                    let high = &self.weights[lower + 1];
                    // Independent sums let the compiler process several taps together.
                    const LANES: usize = 8;
                    let mut sums = [0.0_f32; LANES];
                    for ((samples, low), high) in samples
                        .as_chunks::<LANES>()
                        .0
                        .iter()
                        .zip(low.as_chunks::<LANES>().0)
                        .zip(high.as_chunks::<LANES>().0)
                    {
                        for lane in 0..LANES {
                            let weight = low[lane] + mix * (high[lane] - low[lane]);
                            sums[lane] += samples[lane] * weight;
                        }
                    }
                    let mut value = sums.iter().sum::<f32>();
                    for i in (TAPS / LANES * LANES)..TAPS {
                        let weight = low[i] + mix * (high[i] - low[i]);
                        value += samples[i] * weight;
                    }
                    return value;
                }
            }
        }
        self.read_with(at, input.len(), |i| input[i])
    }
    /// The callback reads one channel at a valid frame index. Edges are extended with zeros.
    #[inline]
    pub fn read_with(&self, at: f64, length: usize, sample: impl Fn(usize) -> f32) -> f32 {
        if !at.is_finite() || at < 0.0 || at >= length as f64 || length > isize::MAX as usize / 4 {
            return 0.0;
        }
        let base = at.floor() as isize;
        let phase = (at - base as f64) * 1024.0;
        let lower = (phase.floor() as usize).min(1023);
        let mix = (phase - lower as f64) as f32;
        let left = TAPS / 2 - 1;
        let mut value = 0.0;
        for i in 0..TAPS {
            let index = base + i as isize - left as isize;
            if index >= 0 && (index as usize) < length {
                let weight = self.weights[lower][i]
                    + mix * (self.weights[lower + 1][i] - self.weights[lower][i]);
                value += sample(index as usize) * weight;
            }
        }
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn compare_scalar<const TAPS: usize>() {
        let sampler = FractionalSinc::<TAPS>::new().unwrap();
        let mut state = 47_u64;
        let mut random = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 40) as f64 / 16777216.0
        };
        let input: Vec<_> = (0..512).map(|_| (random() * 2.0 - 1.0) as f32).collect();
        for at in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 512.0]
            .into_iter()
            .chain((0..4096).map(|_| random() * 512.0))
        {
            let actual = sampler.read(&input, at);
            let expected = sampler.read_with(at, input.len(), |i| input[i]);
            assert!((actual - expected).abs() <= 2e-6);
            if !at.is_finite()
                || at < (TAPS / 2 - 1) as f64
                || at.floor() + (TAPS / 2) as f64 >= input.len() as f64
            {
                assert!(actual.to_bits() == expected.to_bits());
            }
        }
        for length in [0, 1, TAPS - 1] {
            let input = &input[..length];
            for at in [0.0, 0.5, length as f64 - 0.25] {
                assert!(
                    sampler.read(input, at).to_bits()
                        == sampler.read_with(at, input.len(), |i| input[i]).to_bits()
                );
            }
        }
    }
    #[test]
    fn parallel_tap_sums_preserve_scalar_edges_and_even_tap_remainders() {
        compare_scalar::<8>();
        compare_scalar::<10>();
        compare_scalar::<32>();
        compare_scalar::<34>();
        compare_scalar::<64>();
        compare_scalar::<66>();
        compare_scalar::<126>();
        compare_scalar::<128>();
    }
    #[test]
    fn sixty_four_taps_preserve_high_band_reconstruction_at_both_supported_rates() {
        let sampler = FractionalSinc::<64>::new().unwrap();
        for rate in [44100.0, 48000.0] {
            let signal = |at: f64| {
                [8100.0, 12431.0, 16853.0, 19991.0]
                    .iter()
                    .enumerate()
                    .map(|(i, hz)| {
                        (std::f64::consts::TAU * hz * at / rate + i as f64 * 0.37).sin() * 0.1
                    })
                    .sum::<f64>()
            };
            let input: Vec<_> = (0..8192).map(|i| signal(i as f64) as f32).collect();
            for drift in [-0.0002, 0.0, 0.0002] {
                let mut error = 0.0;
                let mut power = 0.0;
                for i in 64..8128 {
                    let at = i as f64 * (1.0 + drift) + 0.37123;
                    let expected = signal(at);
                    error += (sampler.read(&input, at) as f64 - expected).powi(2);
                    power += expected * expected;
                }
                let relative = (error / power).sqrt();
                assert!(relative < 0.0001, "High-band relative error {relative}.");
            }
        }
    }
    #[test]
    fn fractional_sampling_preserves_integer_frames_dc_and_high_band_phase_through_clock_warp() {
        let sampler = FractionalSinc::<32>::new().unwrap();
        let signal = |at: f64| {
            [8100.0, 12431.0, 16853.0, 19991.0]
                .iter()
                .enumerate()
                .map(|(i, hz)| {
                    (std::f64::consts::TAU * hz * at / 48000.0 + i as f64 * 0.37).sin() * 0.1
                })
                .sum::<f64>()
        };
        let input: Vec<_> = (0..8192).map(|i| signal(i as f64) as f32).collect();
        for rate in [-0.0002, 0.0, 0.0002] {
            let mut error = 0.0;
            let mut power = 0.0;
            for i in 64..8128 {
                let at = i as f64 * (1.0 + rate) + 0.37123;
                let expected = signal(at);
                error += (sampler.read(&input, at) as f64 - expected).powi(2);
                power += expected * expected;
            }
            let relative = (error / power).sqrt();
            assert!(
                relative < 0.0003,
                "Fractional sampler relative error {relative}."
            );
        }
        assert!(sampler.read(&input, 100.0) == input[100]);
        for phase in 0..1000 {
            assert!((sampler.read(&[1.0; 100], 50.0 + phase as f64 / 1000.0) - 1.0).abs() < 1e-6);
        }
        assert_eq!(sampler.read(&input, -1.0), 0.0);
        assert!(FractionalSinc::<0>::new().is_err());
    }
}
