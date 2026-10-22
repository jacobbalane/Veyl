use std::{error::Error, f32::consts::PI, fmt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FilterError(pub &'static str);

impl fmt::Display for FilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl Error for FilterError {}

// Filter state contains audio, so it deliberately has no Debug implementation.
#[derive(Clone, Copy)]
pub struct Biquad {
    b: [f32; 3],
    a: [f32; 2],
    state: [f32; 2],
}

impl Biquad {
    pub fn high_pass(rate: u32, cutoff: f32, q: f32) -> Self {
        Self::new(rate, cutoff, q, true)
    }
    pub fn low_pass(rate: u32, cutoff: f32, q: f32) -> Self {
        Self::new(rate, cutoff, q, false)
    }
    /// RBJ high shelf with slope one: https://www.w3.org/TR/audio-eq-cookbook/.
    pub fn high_shelf(rate: u32, cutoff: f32, gain_db: f32) -> Self {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let (sin, cos) = (2.0 * PI * cutoff / rate as f32).sin_cos();
        let beta = 2.0 * a.sqrt() * sin / std::f32::consts::SQRT_2;
        let a0 = (a + 1.0) - (a - 1.0) * cos + beta;
        Self {
            b: [
                a * ((a + 1.0) + (a - 1.0) * cos + beta) / a0,
                -2.0 * a * ((a - 1.0) + (a + 1.0) * cos) / a0,
                a * ((a + 1.0) + (a - 1.0) * cos - beta) / a0,
            ],
            a: [
                2.0 * ((a - 1.0) - (a + 1.0) * cos) / a0,
                ((a + 1.0) - (a - 1.0) * cos - beta) / a0,
            ],
            state: [0.0; 2],
        }
    }
    pub(crate) fn new(rate: u32, cutoff: f32, q: f32, highpass: bool) -> Self {
        let omega = 2.0 * PI * cutoff / rate as f32;
        let (sin, cos) = omega.sin_cos();
        let alpha = sin / (2.0 * q);
        let a0 = 1.0 + alpha;
        let b = if highpass {
            [(1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0]
        } else {
            [(1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0]
        };
        Self {
            b: b.map(|value| value / a0),
            a: [-2.0 * cos / a0, (1.0 - alpha) / a0],
            state: [0.0; 2],
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let output = self.b[0] * input + self.state[0];
        self.state[0] = self.b[1] * input - self.a[0] * output + self.state[1];
        self.state[1] = self.b[2] * input - self.a[1] * output;
        output
    }
}

/// Removes voice frequencies at the source, before audio reaches a link or recorder.
pub struct PrivacyFilter {
    channels: Vec<[Biquad; 4]>,
}

impl PrivacyFilter {
    pub fn new(rate: u32, cutoff: u32, channels: usize) -> Result<Self, FilterError> {
        if ![44_100, 48_000].contains(&rate) {
            return Err(FilterError("The sample rate must be 44100 or 48000 Hz."));
        }
        if ![6000, 7000, 8000].contains(&cutoff) {
            return Err(FilterError(
                "The privacy cutoff must be 6000, 7000 or 8000 Hz.",
            ));
        }
        if !(1..=4).contains(&channels) {
            return Err(FilterError("Audio must have one to four channels."));
        }
        let stages =
            [0.50980, 0.60134, 0.89998, 2.56292].map(|q| Biquad::new(rate, cutoff as f32, q, true));
        Ok(Self {
            channels: vec![stages; channels],
        })
    }

    pub fn process_interleaved(&mut self, samples: &mut [f32]) -> Result<(), FilterError> {
        if !samples.len().is_multiple_of(self.channels.len()) {
            return Err(FilterError("The audio block ends inside a frame."));
        }
        for frame in samples.chunks_exact_mut(self.channels.len()) {
            for (sample, stages) in frame.iter_mut().zip(&mut self.channels) {
                for stage in stages {
                    *sample = stage.process(*sample);
                }
            }
        }
        Ok(())
    }

    pub fn reset(&mut self) {
        for stages in &mut self.channels {
            for stage in stages {
                stage.state = [0.0; 2];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(rate: u32, cutoff: u32, hz: f32) -> f32 {
        let mut samples: Vec<f32> = (0..rate)
            .map(|n| (2.0 * std::f64::consts::PI * hz as f64 * n as f64 / rate as f64).sin() as f32)
            .collect();
        PrivacyFilter::new(rate, cutoff, 1)
            .unwrap()
            .process_interleaved(&mut samples)
            .unwrap();
        let tail = &samples[rate as usize / 2..];
        let power = tail.iter().map(|x| (x * x) as f64).sum::<f64>() / tail.len() as f64;
        (10.0 * (power / 0.5).log10()) as f32
    }
    #[test]
    fn speaker_shelf_has_the_requested_midpoint_and_endpoint_gains() {
        for (hz, expected) in [(1000.0, 0.0), (14000.0, -1.5), (22000.0, -3.0)] {
            let mut filter = Biquad::high_shelf(48000, 14000.0, -3.0);
            let mut power = 0.0;
            for n in 0..48000 {
                let input = (2.0 * std::f64::consts::PI * hz * n as f64 / 48000.0).sin() as f32;
                let output = filter.process(input);
                if n >= 24000 {
                    power += (output * output) as f64;
                }
            }
            let actual = 10.0 * (power / 24000.0 / 0.5).log10();
            assert!(
                (actual - expected).abs() < 0.1,
                "Shelf response at {hz} Hz."
            );
        }
    }

    #[test]
    fn privacy_response_meets_the_specification() {
        for rate in [44_100, 48_000] {
            assert!((response(rate, 8000, 8000.0) + 3.0).abs() < 0.3);
            assert!(response(rate, 8000, 4000.0) <= -40.0);
            assert!(response(rate, 8000, 2000.0) <= -70.0);
            for hz in [11_000.0, 15_000.0, 20_000.0] {
                assert!(response(rate, 8000, hz).abs() < 0.3);
            }
            for cutoff in [6000, 7000] {
                assert!((response(rate, cutoff, cutoff as f32) + 3.0).abs() < 0.3);
            }
        }
    }

    #[test]
    fn channels_are_independent_and_reset_clears_the_tail() {
        let mut filter = PrivacyFilter::new(48_000, 8000, 2).unwrap();
        let mut impulse = vec![0.0; 48_000];
        impulse[0] = 1.0;
        filter.process_interleaved(&mut impulse).unwrap();
        assert!(impulse.iter().all(|sample| sample.is_finite()));
        assert!(impulse
            .as_chunks::<2>()
            .0
            .iter()
            .all(|frame| frame[1] == 0.0));
        assert!(impulse[47_000..].iter().all(|x| x.abs() < 1e-6));
        filter.reset();
        let mut zeros = [0.0; 20];
        filter.process_interleaved(&mut zeros).unwrap();
        assert_eq!(zeros, [0.0; 20]);
    }

    #[test]
    fn block_boundaries_do_not_change_the_output() {
        let input: Vec<f32> = (0..4096).map(|n| (n as f32 * 0.2).sin()).collect();
        let mut whole = input.clone();
        let mut blocks = input;
        PrivacyFilter::new(48_000, 8000, 2)
            .unwrap()
            .process_interleaved(&mut whole)
            .unwrap();
        let mut filter = PrivacyFilter::new(48_000, 8000, 2).unwrap();
        for block in blocks.chunks_mut(64) {
            filter.process_interleaved(block).unwrap();
        }
        assert_eq!(whole, blocks);
        assert!(filter.process_interleaved(&mut [0.0; 3]).is_err());
        assert!(PrivacyFilter::new(16_000, 8000, 1).is_err());
        assert!(PrivacyFilter::new(48_000, 4000, 1).is_err());
        assert!(PrivacyFilter::new(48_000, 8000, 0).is_err());
    }
}
