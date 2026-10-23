use super::*;
use crate::{
    basic::quantize,
    sampling::{convolve, Lanczos, Noise},
    BlockMeta, PokeTruth, Rendered, TruthSample,
};
use veyl_dsp::{
    filters::{Biquad, PrivacyFilter},
    testsound::BrightLoop,
};
const SPEED: f64 = 343.42;
fn db(level: f32) -> f32 {
    10.0_f32.powf(level / 20.0)
}
fn highpass(signal: &[f32], fs: u32) -> Vec<f32> {
    let coefficient = (std::f32::consts::PI * 1000.0 / fs as f32).tan();
    let b0 = 1.0 / (1.0 + coefficient);
    let a1 = (coefficient - 1.0) / (coefficient + 1.0);
    let mut last_input = 0.0;
    let mut last_output = 0.0;
    signal
        .iter()
        .map(|&sample| {
            let out = b0 * (sample - last_input) - a1 * last_output;
            last_input = sample;
            last_output = out;
            out
        })
        .collect()
}
fn music(scene: &Scene, frames: usize) -> io::Result<Vec<f32>> {
    let mut result = vec![0.0; frames * 2];
    let source = match &scene.music {
        Music::Silence => return Ok(result),
        Music::Samples(samples) => samples.clone(),
        Music::Bright { seed } | Music::Compressed16k { seed } | Music::Gappy { seed } => {
            BrightLoop::new(scene.fs, *seed)
                .map_err(io::Error::other)?
                .frames()
                .to_vec()
        }
        Music::Dull { seed } => BrightLoop::dull(scene.fs, *seed)
            .map_err(io::Error::other)?
            .frames()
            .to_vec(),
    };
    for (i, frame) in result.as_chunks_mut::<2>().0.iter_mut().enumerate() {
        if matches!(scene.music, Music::Gappy { .. }) && (i / scene.fs as usize) % 4 == 3 {
            continue;
        }
        frame.copy_from_slice(&source[i % source.len()]);
    }
    if matches!(scene.music, Music::Compressed16k { .. }) {
        let q = [0.5097956, 0.6013449, 0.8999762, 2.5629154];
        let mut sections = q.map(|q| [Biquad::low_pass(scene.fs, 16000.0, q); 2]);
        for frame in result.as_chunks_mut::<2>().0 {
            for (channel, value) in frame.iter_mut().enumerate() {
                for section in &mut sections {
                    *value = section[channel].process(*value);
                }
            }
        }
    }
    Ok(result)
}
fn state(scene: &Scene, seconds: f64) -> (f32, bool, bool) {
    let mut volume = scene.volume;
    let mut external = false;
    let mut blocked = false;
    for event in &scene.events {
        if event.at() > seconds {
            break;
        }
        match event {
            SceneEvent::Volume { gain, .. } => volume = scene.volume * gain,
            SceneEvent::Route { external: next, .. } => external = *next,
            SceneEvent::CaptureBlocked { blocked: next, .. } => blocked = *next,
        }
    }
    (volume, external, blocked)
}
fn room_noise(scene: &Scene, frames: usize, channel: usize) -> Vec<f32> {
    let mut result = vec![0.0; frames];
    for (index, kind) in scene.room_noise.iter().enumerate() {
        let mut noise = Noise::new(scene.seed ^ 0xf00d_1234_5678_9012 ^ (index * 11) as u64);
        match kind {
            RoomNoise::Typing { rate_hz } if *rate_hz > 0.0 => {
                let mut at = 0.0;
                while at < scene.seconds as f64 {
                    let uniform = ((noise.sample() as f64 + 1.0) * 0.5).clamp(1e-9, 1.0 - 1e-9);
                    at += -uniform.ln() / rate_hz;
                    let first = (at * scene.fs as f64) as usize;
                    let length = scene.fs as usize * 2 / 1000;
                    for i in 0..length {
                        if let Some(sample) = result.get_mut(first + i) {
                            *sample += noise.sample()
                                * db(-30.0)
                                * (-(i as f32) / (scene.fs as f32 * 0.0005)).exp();
                        }
                    }
                }
            }
            RoomNoise::Fan => {
                let mut pink = [0.0_f32; 3];
                let mut filter =
                    Biquad::low_pass(scene.fs, 2000.0, std::f32::consts::FRAC_1_SQRT_2);
                for sample in &mut result {
                    let white = noise.sample();
                    for (i, value) in pink.iter_mut().enumerate() {
                        let alpha = [0.997, 0.98, 0.8][i];
                        *value = alpha * *value + (1.0 - alpha) * white;
                    }
                    *sample += filter.process(pink.iter().sum::<f32>() * 3.0) * db(-50.0);
                }
            }
            RoomNoise::Speech { dbfs } => {
                let mut hi = Biquad::high_pass(scene.fs, 100.0, std::f32::consts::FRAC_1_SQRT_2);
                let mut lo = Biquad::low_pass(scene.fs, 4000.0, std::f32::consts::FRAC_1_SQRT_2);
                for (i, sample) in result.iter_mut().enumerate() {
                    let envelope = 0.4
                        + 0.6
                            * (std::f32::consts::TAU * 3.0 * i as f32 / scene.fs as f32)
                                .sin()
                                .powi(2);
                    *sample += lo.process(hi.process(noise.sample()))
                        * db(*dbfs)
                        * 3.0_f32.sqrt()
                        * envelope;
                }
            }
            _ => {}
        }
    }
    let gain = 0.95_f32.powi(channel as i32);
    for sample in &mut result {
        *sample *= gain;
    }
    result
}
fn echo_distance(scene: &Scene, point: [f64; 3]) -> f64 {
    let speaker = scene.speakers[0].position;
    let mic = scene.mics[0].position;
    (distance(speaker, point) + distance(point, mic) - distance(speaker, mic)) * 0.5
}
pub fn render(scene: &Scene) -> io::Result<Rendered> {
    scene.validate()?;
    let blocks_count = (scene.seconds as f64 * 100.0).round() as usize;
    let frames = blocks_count * (scene.fs / 100) as usize;
    let mut reference = music(scene, frames)?;
    let channels = scene.mics.len();
    let mut mic = vec![0.0; frames * channels];
    let scale = 1.0 + scene.clock_drift_ppm * 1e-6;
    let sinc = Lanczos::new();
    for (speaker_index, transducer) in scene.speakers.iter().enumerate() {
        let mut hi = Biquad::high_pass(
            scene.fs,
            scene.speaker_model.highpass_hz,
            std::f32::consts::FRAC_1_SQRT_2,
        );
        let mut shelf = Biquad::high_shelf(
            scene.fs,
            scene.speaker_model.shelf_hz,
            scene.speaker_model.shelf_db,
        );
        let speaker: Vec<f32> = reference
            .as_chunks::<2>()
            .0
            .iter()
            .enumerate()
            .map(|(i, frame)| {
                let sample = match transducer.channel {
                    Channel::Left => frame[0],
                    Channel::Right => frame[1],
                    Channel::Mix => (frame[0] + frame[1]) * 0.5,
                };
                let (volume, external, _) = state(scene, i as f64 / scene.fs as f64);
                let out = (shelf.process(hi.process(sample)) * volume).tanh();
                if external {
                    0.0
                } else {
                    out
                }
            })
            .collect();
        let coloured = highpass(&speaker, scene.fs);
        for (channel, microphone) in scene.mics.iter().enumerate() {
            let direct = distance(transducer.position, microphone.position);
            let delay = (scene.output_latency_ms / 1000.0 + direct / SPEED) * scene.fs as f64;
            let mut noise = Noise::new(
                scene.seed ^ 0xc0ff_ee12_3456_7890 ^ ((speaker_index * 7 + channel) as u64),
            );
            let mut impulse: Vec<_> = (0..scene.fs as usize * 30 / 1000)
                .map(|i| noise.sample() * (-(i as f32) / (scene.fs as f32 * 0.01)).exp())
                .collect();
            let norm = impulse.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-12);
            for value in &mut impulse {
                *value *= db(-30.0) / norm;
            }
            let diffuse = convolve(&speaker, &impulse);
            let mut paths = vec![(0.0_f64, 0.0_f32, 0.0_f64, 0.0_f32); scene.movers.len()];
            for start in (0..frames).step_by(16) {
                for (index, mover) in scene.movers.iter().enumerate() {
                    let path = |frame: usize| {
                        let point = mover.position(frame as f64 * scale / scene.fs as f64);
                        let a = distance(transducer.position, point).max(0.005);
                        let b = distance(point, microphone.position).max(0.005);
                        (
                            (scene.output_latency_ms / 1000.0 + (a + b) / SPEED) * scene.fs as f64,
                            (mover.gain() * (0.05 / a) * (0.05 / b)) as f32,
                        )
                    };
                    let near = path(start);
                    let far = path(start + 16);
                    paths[index] = (near.0, near.1, far.0, far.1);
                }
                for offset in 0..16.min(frames - start) {
                    let frame = start + offset;
                    let at = frame as f64 * scale;
                    let fraction = offset as f64 / 16.0;
                    let mut value =
                        sinc.read(&speaker, at - delay) + sinc.read(&diffuse, at - delay);
                    for echo in &scene.statics {
                        value += echo.gain
                            * sinc.read(
                                match echo.colour {
                                    Colour::Flat => &speaker,
                                    Colour::HighPass1k => &coloured,
                                },
                                at - delay - echo.extra_path_m / SPEED * scene.fs as f64,
                            );
                    }
                    for &(near_delay, near_gain, far_delay, far_gain) in &paths {
                        let delay = near_delay + (far_delay - near_delay) * fraction;
                        let gain = near_gain + (far_gain - near_gain) * fraction as f32;
                        value += gain * sinc.read(&coloured, at - delay);
                    }
                    mic[frame * channels + channel] += value;
                }
            }
        }
    }
    for channel in 0..channels {
        let room = room_noise(scene, frames, channel);
        let mut noise = Noise::new(scene.seed ^ 0xabcd_5678_1234_9012 ^ channel as u64);
        for (frame, room) in room.into_iter().enumerate() {
            let value = &mut mic[frame * channels + channel];
            *value = (*value + room + noise.sample() * db(scene.mic_noise_dbfs) * 3.0_f32.sqrt())
                .clamp(-1.0, 1.0);
        }
    }
    for (frame, samples) in reference.as_chunks_mut::<2>().0.iter_mut().enumerate() {
        if state(scene, frame as f64 / scene.fs as f64).2 {
            samples.fill(0.0);
        }
    }
    let mut blocks = Vec::with_capacity(blocks_count * 2);
    let mut gap = [false; 2];
    for seq in 0..blocks_count {
        let first_frame = seq as u64 * (scene.fs / 100) as u64;
        let lost = scene.dropouts.iter().any(|&(start, length)| {
            first_frame >= (start * scene.fs as f64).round() as u64
                && first_frame < ((start + length) * scene.fs as f64).round() as u64
        });
        for (stream, signal, channels) in [(0, &reference, 2), (1, &mic, channels)] {
            if lost {
                gap[stream] = true;
                continue;
            }
            let begin = first_frame as usize * channels;
            let samples = &signal[begin..begin + scene.fs as usize / 100 * channels];
            let power = samples
                .iter()
                .map(|&sample| (sample as f64).powi(2))
                .sum::<f64>()
                / samples.len() as f64;
            let peak = samples.iter().map(|x| x.abs()).fold(0.0, f32::max);
            let flags = 4
                + if stream == 0 { 8 } else { 0 }
                + if peak >= 0.99 { 2 } else { 0 }
                + u16::from(gap[stream]);
            gap[stream] = false;
            blocks.push(BlockMeta {
                stream: stream as u8,
                seq: seq as u32,
                first_frame,
                capture_ns: (first_frame as f64 * 1e9 / scene.fs as f64
                    * if stream == 1 { scale } else { 1.0 }) as i64,
                flags,
                pre_rms_dbfs: (10.0 * power.max(1e-12).log10()) as f32,
                pre_peak: peak,
            });
        }
    }
    PrivacyFilter::new(scene.fs, 8000, 2)
        .map_err(io::Error::other)?
        .process_interleaved(&mut reference)
        .map_err(io::Error::other)?;
    PrivacyFilter::new(scene.fs, 8000, channels)
        .map_err(io::Error::other)?
        .process_interleaved(&mut mic)
        .map_err(io::Error::other)?;
    for sample in reference.iter_mut().chain(&mut mic) {
        *sample = quantize(*sample) as f32 / 32768.0;
    }
    let mut labels = vec![];
    let mut truth = Vec::with_capacity(blocks_count * scene.movers.len());
    for (mover_index, mover) in scene.movers.iter().enumerate() {
        for seq in 0..blocks_count {
            let frame = seq as u64 * 480;
            truth.push(TruthSample {
                mover: mover_index,
                frame,
                echo_distance_m: echo_distance(
                    scene,
                    mover.position(frame as f64 * scale / scene.fs as f64),
                ) as f32,
            });
        }
        if let Mover::Pokes(pokes) = mover {
            for poke in pokes {
                let turn_frame = (poke.turn_seconds * scene.fs as f64 / scale).round() as u64;
                if turn_frame >= frames as u64 {
                    continue;
                }
                let closest = [poke.from[0], poke.from[1], poke.from[2] - poke.depth];
                labels.push(PokeTruth {
                    turn_frame,
                    distance_cm: (echo_distance(scene, closest) * 100.0) as f32,
                    depth_cm: ((echo_distance(scene, poke.from) - echo_distance(scene, closest))
                        * 100.0) as f32,
                });
            }
        }
    }
    labels.sort_by_key(|label| label.turn_frame);
    Ok(Rendered {
        sample_rate: scene.fs,
        ref_channels: 2,
        mic_channels: channels as u8,
        reference,
        mic,
        blocks,
        labels,
        truth,
        events: scene.events.clone(),
    })
}
