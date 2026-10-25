//! Development probe for the processing cost of each candidate: milliseconds of engine time
//! per second of audio, from ten seconds measured after ten seconds of warm-up, on identical
//! 2-reference, 2-microphone input at 48 kHz. Wall-clock time on a shared machine varies, so it
//! prints the median and range of five repetitions. It never writes audio.
use std::{error::Error, time::Instant};
use veyl_dsp::{
    config::EngineConfig, BlockFlags, BlockIn, Engine, SoundSource, SourceStatus, Stream,
    StreamFormat,
};
use veyl_sim::{basic::quantize, scenarios, scene};

fn main() -> Result<(), Box<dyn Error>> {
    let rendered = scene::render(&scenarios::scene("poke_basic", 1, Some(20.0))?)?;
    let pcm: [Vec<i16>; 2] = [
        rendered.reference.iter().map(|v| quantize(*v)).collect(),
        rendered.mic.iter().map(|v| quantize(*v)).collect(),
    ];
    let residual = EngineConfig {
        infer_single_path: true,
        resample_reference: true,
        residual_motion: true,
        floor_relative: 1e-8,
        impulse_ratio: 12.0,
        ..EngineConfig::default()
    };
    let candidates = [
        ("defaults", EngineConfig::default()),
        ("residual (0020)", residual.clone()),
        (
            "short change (0021)",
            EngineConfig {
                impulse_relative: true,
                change_window: 1024,
                change_smooth_bins: 8,
                track_gate_cm: 4.0,
                ..residual
            },
        ),
    ];
    for (name, config) in candidates {
        let mut timings = vec![];
        for _ in 0..5 {
            let mut engine = Engine::new(
                config.clone(),
                StreamFormat {
                    sample_rate: rendered.sample_rate,
                    ref_channels: rendered.ref_channels,
                    mic_channels: rendered.mic_channels,
                },
            )?;
            engine.set_source_status(SourceStatus {
                capture_running: true,
                source: SoundSource::TestSound,
                ..SourceStatus::default()
            })?;
            let mut events = Vec::with_capacity(64);
            let mut elapsed = 0.0;
            for meta in &rendered.blocks {
                let channels = if meta.stream == 0 {
                    rendered.ref_channels
                } else {
                    rendered.mic_channels
                } as usize;
                let first = meta.first_frame as usize * channels;
                let block = BlockIn {
                    stream: if meta.stream == 0 {
                        Stream::Reference
                    } else {
                        Stream::Microphone
                    },
                    first_frame: meta.first_frame,
                    capture_ns: Some(meta.capture_ns),
                    flags: BlockFlags(meta.flags),
                    pre_rms_dbfs: meta.pre_rms_dbfs,
                    pre_peak: meta.pre_peak,
                    samples: &pcm[meta.stream as usize][first..first + 480 * channels],
                };
                let start = Instant::now();
                engine.push(block)?;
                events.clear();
                engine.process(&mut events)?;
                if meta.first_frame >= 480_000 {
                    elapsed += start.elapsed().as_secs_f64();
                }
            }
            timings.push(elapsed * 1000.0 / 10.0);
        }
        timings.sort_by(f64::total_cmp);
        println!(
            "{name}: median {:.2} ms per second of audio (range {:.2}-{:.2})",
            timings[2], timings[0], timings[4]
        );
    }
    Ok(())
}
