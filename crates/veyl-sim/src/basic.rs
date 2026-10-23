//! The default phone scene supplies real acoustic paths, rather than fabricated echo features.
use crate::sampling::{convolve, Lanczos, Noise};
use serde::{Deserialize, Serialize};
use std::{f32::consts::PI, io};
use veyl_dsp::{
    filters::{Biquad, PrivacyFilter},
    testsound::BrightLoop,
};

const FS: u32 = 48000;
const SPEED: f64 = 343.42;
const SPEAKER: [f64; 3] = [0.02, -0.075, 0.0];
const MICS: [[f64; 3]; 2] = [[-0.02, -0.075, 0.0], [0.01, 0.075, 0.0]];

#[derive(Clone, Serialize, Deserialize)]
pub struct BlockMeta {
    pub stream: u8,
    pub seq: u32,
    pub first_frame: u64,
    pub capture_ns: i64,
    pub flags: u16,
    pub pre_rms_dbfs: f32,
    pub pre_peak: f32,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct PokeTruth {
    pub turn_frame: u64,
    pub distance_cm: f32,
    pub depth_cm: f32,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct TruthSample {
    #[serde(default)]
    pub mover: usize,
    pub frame: u64,
    pub echo_distance_m: f32,
}
pub struct Rendered {
    pub events: Vec<crate::scene::SceneEvent>,
    pub sample_rate: u32,
    pub ref_channels: u8,
    pub mic_channels: u8,
    /// Filtered, quantized PCM represented as normalized floats for offline processing.
    pub reference: Vec<f32>,
    pub mic: Vec<f32>,
    pub blocks: Vec<BlockMeta>,
    pub labels: Vec<PokeTruth>,
    pub truth: Vec<TruthSample>,
}
impl Rendered {
    pub fn samples(&self, block: &BlockMeta) -> Option<&[f32]> {
        let (audio, channels) = match block.stream {
            0 => (&self.reference, self.ref_channels),
            1 => (&self.mic, self.mic_channels),
            _ => return None,
        };
        let start = usize::try_from(block.first_frame)
            .ok()?
            .checked_mul(channels as usize)?;
        let count = (self.sample_rate / 100) as usize * channels as usize;
        audio.get(start..start.checked_add(count)?)
    }
}
pub fn quantize(value: f32) -> i16 {
    (value * 32768.0).round().clamp(-32768.0, 32767.0) as i16
}
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter()
        .zip(b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}
fn height(time: f64) -> f64 {
    for index in 0..10 {
        let turn = 5.0 + index as f64 * 3.0;
        let start = turn - 0.18;
        if (start..turn - 0.03).contains(&time) {
            return 0.2 - 0.05 * (time - start) / 0.15;
        }
        if (turn - 0.03..turn).contains(&time) {
            return 0.15;
        }
        if (turn..turn + 0.2).contains(&time) {
            return 0.15 + 0.05 * (time - turn) / 0.2;
        }
    }
    0.2
}
fn echo_distance(z: f64) -> f64 {
    (distance(SPEAKER, [0.0, 0.0, z]) + distance([0.0, 0.0, z], MICS[0])
        - distance(SPEAKER, MICS[0]))
        / 2.0
}
fn stats(samples: &[f32]) -> (f32, f32) {
    let power = samples
        .iter()
        .map(|&value| (value as f64).powi(2))
        .sum::<f64>()
        / samples.len().max(1) as f64;
    (
        (10.0 * power.max(1e-12).log10()) as f32,
        samples.iter().map(|value| value.abs()).fold(0.0, f32::max),
    )
}

/// Forty seconds contains ten pokes. Shorter inputs are useful for deterministic unit checks.
pub fn poke_basic(seed: u64, seconds: f32) -> io::Result<Rendered> {
    if seed == 0 || !seconds.is_finite() || !(0.01..=600.0).contains(&seconds) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Use a nonzero seed and 0.01 to 600 seconds.",
        ));
    }
    let count = (seconds as f64 * 100.0).round() as usize;
    let frames = count * (FS / 100) as usize;
    let loop_sound = BrightLoop::new(FS, seed).map_err(io::Error::other)?;
    let mut reference = Vec::with_capacity(frames * 2);
    let mut speaker = Vec::with_capacity(frames);
    let mut highpass = Biquad::high_pass(FS, 300.0, std::f32::consts::FRAC_1_SQRT_2);
    let mut shelf = Biquad::high_shelf(FS, 14000.0, -3.0);
    for frame in 0..frames {
        let [left, right] = loop_sound.frames()[frame % loop_sound.frames().len()];
        reference.extend([left, right]);
        let eq = shelf.process(highpass.process((left + right) * 0.5));
        speaker.push((eq * 0.5).tanh());
    }
    let coefficient = (PI * 1000.0 / FS as f32).tan();
    let b0 = 1.0 / (1.0 + coefficient);
    let a1 = (coefficient - 1.0) / (coefficient + 1.0);
    let mut last_input = 0.0;
    let mut last_output = 0.0;
    let hand_signal: Vec<_> = speaker
        .iter()
        .map(|&input| {
            let value = b0 * (input - last_input) - a1 * last_output;
            last_input = input;
            last_output = value;
            value
        })
        .collect();
    let sinc = Lanczos::new();
    let mut mic = vec![0.0; frames * 2];
    for (channel, &position) in MICS.iter().enumerate() {
        let direct = distance(SPEAKER, position);
        let delay = (0.04 + direct / SPEED) * FS as f64;
        let mut noise = Noise::new(seed ^ (0xc0ff_ee12_3456_7890 + channel as u64));
        let mut impulse: Vec<_> = (0..(FS as usize * 30 / 1000))
            .map(|n| noise.sample() * (-(n as f32) / (FS as f32 * 0.01)).exp())
            .collect();
        let norm = (impulse.iter().map(|&x| x * x).sum::<f32>()).sqrt();
        for value in &mut impulse {
            *value *= 10.0_f32.powf(-30.0 / 20.0) / norm;
        }
        let diffuse = convolve(&speaker, &impulse);
        let self_noise = 10.0_f32.powf(-85.0 / 20.0) * 3.0_f32.sqrt();
        for start in (0..frames).step_by(16) {
            let path = |frame: usize| {
                let point = [0.0, 0.0, height(frame as f64 / FS as f64)];
                let s = distance(SPEAKER, point);
                let m = distance(point, position);
                (
                    (0.04 + (s + m) / SPEED) * FS as f64,
                    (0.15 * (0.05 / s) * (0.05 / m)) as f32,
                )
            };
            let (near_delay, near_gain) = path(start);
            let (far_delay, far_gain) = path(start + 16);
            for offset in 0..16.min(frames - start) {
                let frame = start + offset;
                let fraction = offset as f64 / 16.0;
                let hand_delay = near_delay + (far_delay - near_delay) * fraction;
                let gain = near_gain + (far_gain - near_gain) * fraction as f32;
                let at = frame as f64;
                let value = sinc.read(&speaker, at - delay)
                    + 0.3 * sinc.read(&speaker, at - delay - 0.03 / SPEED * FS as f64)
                    + 0.05 * sinc.read(&speaker, at - delay - 0.8 / SPEED * FS as f64)
                    + sinc.read(&diffuse, at - delay)
                    + gain * sinc.read(&hand_signal, at - hand_delay)
                    + noise.sample() * self_noise;
                mic[frame * 2 + channel] = value.clamp(-1.0, 1.0);
            }
        }
    }
    let mut blocks = Vec::with_capacity(count * 2);
    let mut truth = Vec::with_capacity(count);
    for seq in 0..count {
        let first_frame = seq as u64 * (FS / 100) as u64;
        for stream in 0..2 {
            let signal = if stream == 0 { &reference } else { &mic };
            let start = first_frame as usize * 2;
            let (pre_rms_dbfs, pre_peak) = stats(&signal[start..start + (FS / 100) as usize * 2]);
            blocks.push(BlockMeta {
                stream,
                seq: seq as u32,
                first_frame,
                capture_ns: (first_frame * 1_000_000_000 / FS as u64) as i64,
                flags: 4 + if stream == 0 { 8 } else { 0 } + if pre_peak >= 0.99 { 2 } else { 0 },
                pre_rms_dbfs,
                pre_peak,
            });
        }
        truth.push(TruthSample {
            mover: 0,
            frame: first_frame,
            echo_distance_m: echo_distance(height(first_frame as f64 / FS as f64)) as f32,
        });
    }
    for signal in [&mut reference, &mut mic] {
        PrivacyFilter::new(FS, 8000, 2)
            .map_err(io::Error::other)?
            .process_interleaved(signal)
            .map_err(io::Error::other)?;
        signal
            .iter_mut()
            .for_each(|value| *value = quantize(*value) as f32 / 32768.0);
    }
    let labels = (0..10)
        .filter_map(|index| {
            let time = 5.0 + index as f64 * 3.0;
            let turn_frame = (time * FS as f64) as u64;
            (turn_frame < (frames as u64)).then(|| PokeTruth {
                turn_frame,
                distance_cm: (echo_distance(0.15) * 100.0) as f32,
                depth_cm: ((echo_distance(0.2) - echo_distance(0.15)) * 100.0) as f32,
            })
        })
        .collect();
    Ok(Rendered {
        events: vec![],
        sample_rate: FS,
        ref_channels: 2,
        mic_channels: 2,
        reference,
        mic,
        blocks,
        labels,
        truth,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn geometry_and_turn_labels_follow_the_approach_hold_and_retreat() {
        assert!((height(4.7) - 0.2).abs() < 1e-9);
        assert!((height(4.98) - 0.15).abs() < 1e-9);
        assert!((height(5.1) - 0.175).abs() < 1e-9);
        assert!((height(5.21) - 0.2).abs() < 1e-9);
        assert!(echo_distance(0.15) < echo_distance(0.2));
        assert!((0.04..0.05).contains(&(echo_distance(0.2) - echo_distance(0.15))));
    }
    #[test]
    fn generated_pcm_is_repeatable_filtered_and_has_complete_metadata() {
        let first = poke_basic(1, 0.5).unwrap();
        let second = poke_basic(1, 0.5).unwrap();
        assert!(first.reference == second.reference && first.mic == second.mic);
        assert_eq!(first.blocks.len(), 100);
        assert_eq!(first.reference.len(), 48000);
        assert_eq!(first.mic.len(), 48000);
        assert!(first
            .reference
            .iter()
            .chain(&first.mic)
            .all(|value| value.is_finite() && (-1.0..1.0).contains(value)));
        let pre = first
            .blocks
            .iter()
            .filter(|block| block.stream == 0)
            .map(|block| 10.0_f64.powf(block.pre_rms_dbfs as f64 / 10.0))
            .sum::<f64>()
            / 50.0;
        let post = first
            .reference
            .iter()
            .map(|&value| (value as f64).powi(2))
            .sum::<f64>()
            / first.reference.len() as f64;
        assert!(pre > post * 10.0);
        for block in &first.blocks {
            assert_eq!(first.samples(block).unwrap().len(), 960);
            assert_eq!(block.first_frame, block.seq as u64 * 480);
            assert!(block.pre_rms_dbfs.is_finite());
        }
        assert!(poke_basic(0, 1.0).is_err());
        assert!(poke_basic(1, f32::NAN).is_err());
    }
    #[test]
    fn forty_seconds_contains_ten_truth_pokes_and_eight_thousand_wire_blocks() {
        let rendered = poke_basic(2, 40.0).unwrap();
        assert_eq!(rendered.labels.len(), 10);
        assert_eq!(rendered.blocks.len(), 8000);
        assert_eq!(rendered.truth.len(), 4000);
        assert_eq!(rendered.labels[0].turn_frame, 5 * 48000);
        assert_eq!(rendered.labels[9].turn_frame, 32 * 48000);
        assert!(rendered
            .labels
            .iter()
            .all(|label| (4.0..5.0).contains(&label.depth_cm)));
    }
}
