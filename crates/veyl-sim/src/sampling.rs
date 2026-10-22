use rustfft::{num_complex::Complex, FftPlanner};
use std::f64::consts::PI;

pub(crate) struct Noise(u64);
impl Noise {
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x5eed_5eed_5eed_5eed
        } else {
            seed
        })
    }
    pub fn sample(&mut self) -> f32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let word = self.0.wrapping_mul(0x2545_f491_4f6c_dd1d);
        ((word >> 40) as f32 / 8_388_608.0) - 1.0
    }
}

/// Eight-tap Lanczos (a=4). Phase-table error is under 1/1024 of a sample.
pub(crate) struct Lanczos(Vec<[f32; 8]>);
impl Lanczos {
    pub fn new() -> Self {
        Self(
            (0..1024)
                .map(|phase| {
                    if phase == 0 {
                        return [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
                    }
                    let fraction = phase as f64 / 1024.0;
                    let sinc = |x: f64| {
                        if x.abs() < 1e-12 {
                            1.0
                        } else {
                            (PI * x).sin() / (PI * x)
                        }
                    };
                    let mut weights = std::array::from_fn::<_, 8, _>(|i| {
                        let x = fraction - (i as f64 - 3.0);
                        if x.abs() < 4.0 {
                            (sinc(x) * sinc(x / 4.0)) as f32
                        } else {
                            0.0
                        }
                    });
                    let sum = weights.iter().sum::<f32>();
                    weights.iter_mut().for_each(|weight| *weight /= sum);
                    weights
                })
                .collect(),
        )
    }
    pub fn read(&self, input: &[f32], at: f64) -> f32 {
        if at < 0.0 || at >= input.len() as f64 {
            return 0.0;
        }
        let base = at.floor() as isize;
        let phase = ((at - base as f64) * 1024.0) as usize;
        let mut value = 0.0;
        for (i, weight) in self.0[phase.min(1023)].iter().enumerate() {
            let index = base + i as isize - 3;
            if index >= 0 && (index as usize) < input.len() {
                value += input[index as usize] * weight;
            }
        }
        value
    }
}

/// Overlap-add keeps the 30 ms diffuse room response practical for long scenes.
pub(crate) fn convolve(input: &[f32], kernel: &[f32]) -> Vec<f32> {
    let size = (2048 + kernel.len() - 1).next_power_of_two();
    let chunk = 2048;
    let mut planner = FftPlanner::<f32>::new();
    let forward = planner.plan_fft_forward(size);
    let inverse = planner.plan_fft_inverse(size);
    let mut impulse = vec![Complex::default(); size];
    for (target, &sample) in impulse.iter_mut().zip(kernel) {
        target.re = sample;
    }
    let mut scratch = vec![
        Complex::default();
        forward
            .get_inplace_scratch_len()
            .max(inverse.get_inplace_scratch_len())
    ];
    forward.process_with_scratch(&mut impulse, &mut scratch);
    let mut buffer = vec![Complex::default(); size];
    let mut output = vec![0.0; input.len()];
    for (index, block) in input.chunks(chunk).enumerate() {
        buffer.fill(Complex::default());
        for (target, &sample) in buffer.iter_mut().zip(block) {
            target.re = sample;
        }
        forward.process_with_scratch(&mut buffer, &mut scratch);
        for (value, &coefficient) in buffer.iter_mut().zip(&impulse) {
            *value *= coefficient;
        }
        inverse.process_with_scratch(&mut buffer, &mut scratch);
        let start = index * chunk;
        let count = (block.len() + kernel.len() - 1).min(output.len() - start);
        for offset in 0..count {
            output[start + offset] += buffer[offset].re / size as f32;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractional_sampling_preserves_dc_integer_samples_and_linear_position() {
        let sinc = Lanczos::new();
        let input: Vec<_> = (0..100).map(|n| n as f32).collect();
        assert_eq!(sinc.read(&input, 20.0), 20.0);
        assert!((sinc.read(&input, 20.5) - 20.5).abs() < 1e-5);
        for phase in 0..100 {
            assert!((sinc.read(&[1.0; 100], 20.0 + phase as f64 / 100.0) - 1.0).abs() < 1e-5);
        }
        assert_eq!(sinc.read(&input, -1.0), 0.0);
    }
    #[test]
    fn convolution_matches_direct_filtering_across_fft_block_edges() {
        let input: Vec<_> = (0..6000).map(|n| (n as f32 * 0.17).sin()).collect();
        let kernel = [0.5, 0.3, -0.1];
        let actual = convolve(&input, &kernel);
        for n in 0..input.len() {
            let expected = kernel
                .iter()
                .enumerate()
                .filter(|(i, _)| *i <= n)
                .map(|(i, &k)| input[n - i] * k)
                .sum::<f32>();
            assert!(
                (actual[n] - expected).abs() < 1e-5,
                "Convolution at frame {n}."
            );
        }
    }
}
