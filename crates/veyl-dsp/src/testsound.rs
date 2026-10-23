use crate::filters::{Biquad, FilterError};
use std::f32::consts::TAU;

pub const DEFAULT_SEED: u64 = 0x5EED_5EED_5EED_5EED;

struct Noise(u64);
impl Noise {
    fn sample(&mut self) -> f32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let word = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D);
        ((word >> 40) as f32 / 8_388_608.0) - 1.0
    }
}

/// A generated 16-second stereo loop. No recorded music or microphone audio is used.
pub struct BrightLoop {
    samples: Vec<[f32; 2]>,
    cursor: usize,
    sample_rate: u32,
}

fn db(level: f32) -> f32 {
    10.0_f32.powf(level / 20.0)
}

fn pad(time: f32, chord: usize, ceiling: f32) -> f32 {
    let notes = [[60, 64, 67], [57, 60, 64], [53, 57, 60], [55, 59, 62]];
    notes[chord]
        .iter()
        .map(|note| {
            // Rounding by 1/16 Hz makes each oscillator meet itself at the loop boundary.
            let hz = (440.0 * 2.0_f32.powf((*note as f32 - 69.0) / 12.0) * 16.0).round() / 16.0;
            (1..=(ceiling / hz) as u32)
                .map(|harmonic| (TAU * hz * harmonic as f32 * time).sin() / harmonic as f32)
                .sum::<f32>()
        })
        .sum::<f32>()
        / 3.0
}

impl BrightLoop {
    pub fn new(sample_rate: u32, seed: u64) -> Result<Self, FilterError> {
        Self::render(sample_rate, seed, true)
    }
    pub fn dull(sample_rate: u32, seed: u64) -> Result<Self, FilterError> {
        Self::render(sample_rate, seed, false)
    }
    fn render(sample_rate: u32, seed: u64, bright: bool) -> Result<Self, FilterError> {
        if ![44_100, 48_000].contains(&sample_rate) || seed == 0 {
            return Err(FilterError(
                "Test sound needs 44100 or 48000 Hz and a nonzero seed.",
            ));
        }
        let count = sample_rate as usize * 16;
        let mut noise = Noise(seed);
        let raw: Vec<[f32; 2]> = (0..count)
            .map(|_| [noise.sample(), noise.sample()])
            .collect();
        let mut hats = [[Biquad::new(sample_rate, 7000.0, 0.70710677, true); 2]; 2];
        let mut snare = Biquad::new(sample_rate, 1000.0, 0.70710677, true);
        let mut shimmer = [[
            Biquad::new(sample_rate, 9000.0, 0.70710677, true),
            Biquad::new(sample_rate, 18000.0, 0.70710677, false),
        ]; 2];
        // One cycle warms the filters; the second starts with the periodic state.
        for input in &raw {
            snare.process(input[0]);
            for channel in 0..2 {
                let value = hats[channel][0].process(input[channel]);
                hats[channel][1].process(value);
                let value = shimmer[channel][0].process(input[channel]);
                shimmer[channel][1].process(value);
            }
        }
        let mut samples = Vec::with_capacity(count);
        for (index, input) in raw.iter().enumerate() {
            let t = index as f32 / sample_rate as f32;
            let kick_age = t % 1.0;
            let sweep = kick_age.min(0.12);
            let kick_phase =
                110.0 * sweep - (65.0 / 0.24) * sweep * sweep + 45.0 * (kick_age - sweep);
            let kick = db(-8.0) * (-kick_age / 0.060).exp() * (TAU * kick_phase).sin();
            let snare_age = (t - 0.5).rem_euclid(1.0);
            let snare_value = db(-12.0)
                * (snare.process(input[0]) * (-snare_age / 0.070).exp()
                    + 0.3 * (TAU * 190.0 * snare_age).sin() * (-snare_age / 0.040).exp());
            let hat_age = t % 0.25;
            let open_age = (t - 1.75).rem_euclid(2.0);
            let hat_envelope =
                db(-16.0) * (-hat_age / 0.025).exp() + db(-18.0) * (-open_age / 0.120).exp();
            let chord = (t / 4.0) as usize;
            let fade = ((t % 4.0) / 0.1).min(1.0);
            let ceiling = if bright { 5000.0 } else { 3000.0 };
            let pad_value = 0.19
                * (pad(t, chord, ceiling) * fade + pad(t, (chord + 3) % 4, ceiling) * (1.0 - fade));
            let mut frame = [0.0; 2];
            for channel in 0..2 {
                let hat = hats[channel][0].process(input[channel]);
                let hat = hats[channel][1].process(hat);
                let shimmer_value = shimmer[channel][0].process(input[channel]);
                let shimmer_value = shimmer[channel][1].process(shimmer_value);
                let pan = if ((t / 0.25) as usize + channel).is_multiple_of(2) {
                    0.65_f32.sqrt()
                } else {
                    0.35_f32.sqrt()
                };
                let mix = kick
                    + snare_value
                    + pad_value
                    + if bright {
                        hat * hat_envelope * pan
                    } else {
                        0.0
                    }
                    + if bright {
                        shimmer_value * db(-36.0 + 3.0 * (TAU * 0.25 * t).sin()) * 2.4
                    } else {
                        0.0
                    };
                frame[channel] = (1.2 * mix).tanh() / 1.2_f32.tanh();
            }
            samples.push(frame);
        }
        let peak = samples
            .iter()
            .flatten()
            .fold(0.0_f32, |max, x| max.max(x.abs()));
        for value in samples.iter_mut().flatten() {
            *value *= db(-1.0) / peak;
        }
        Ok(Self {
            samples,
            cursor: 0,
            sample_rate,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    pub fn frames(&self) -> &[[f32; 2]] {
        &self.samples
    }

    pub fn next_frame(&mut self) -> [f32; 2] {
        let frame = self.samples[self.cursor];
        self.cursor = (self.cursor + 1) % self.samples.len();
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustfft::{num_complex::Complex, FftPlanner};

    #[test]
    fn repeatable_finite_stereo_and_looping() {
        let mut first = BrightLoop::new(48_000, DEFAULT_SEED).unwrap();
        let second = BrightLoop::new(48_000, DEFAULT_SEED).unwrap();
        assert!(first.frames() == second.frames());
        assert_eq!(first.frames().len(), 16 * 48_000);
        assert!(first
            .frames()
            .iter()
            .flatten()
            .all(|x| x.is_finite() && x.abs() <= db(-1.0) + 1e-6));
        assert!(first.frames().iter().any(|frame| frame[0] != frame[1]));
        for _ in 0..first.frames().len() {
            first.next_frame();
        }
        assert_eq!(first.next_frame(), second.frames()[0]);
        assert!(BrightLoop::new(16_000, DEFAULT_SEED).is_err());
        assert!(BrightLoop::new(48_000, 0).is_err());
    }

    #[test]
    fn sensing_band_has_enough_energy_at_both_rates() {
        for rate in [44_100, 48_000] {
            let sound = BrightLoop::new(rate, DEFAULT_SEED).unwrap();
            let fft = FftPlanner::new().plan_fft_forward(4096);
            let mut band = 0.0_f64;
            let mut total = 0.0_f64;
            for window in sound.frames().as_chunks::<4096>().0 {
                for channel in 0..2 {
                    let mut spectrum: Vec<Complex<f32>> = window
                        .iter()
                        .enumerate()
                        .map(|(i, frame)| {
                            Complex::new(
                                frame[channel] * (0.5 - 0.5 * (TAU * i as f32 / 4096.0).cos()),
                                0.0,
                            )
                        })
                        .collect();
                    fft.process(&mut spectrum);
                    for (bin, value) in spectrum.iter().enumerate().take(2049).skip(1) {
                        let power = value.norm_sqr() as f64;
                        total += power;
                        let hz = bin as f32 * rate as f32 / 4096.0;
                        if (8000.0..=20000.0).contains(&hz) {
                            band += power;
                        }
                    }
                }
            }
            let ratio = 10.0 * (band / total).log10();
            assert!(
                (-20.0..=-8.0).contains(&ratio),
                "{rate} Hz: band energy {ratio:.2} dB"
            );
        }
    }
}
