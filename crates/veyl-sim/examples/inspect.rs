//! Local tuning probe. It prints derived measurements and never writes audio.
use std::{env, error::Error};
use veyl_dsp::{
    config::EngineConfig, BlockFlags, BlockIn, Engine, EngineEvent, Stream, StreamFormat,
};
use veyl_sim::{basic::quantize, scenarios, scene};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() < 5 {
        return Err(
            "Use: inspect SCENE SECONDS mono|original FAST_SECONDS FLOOR_RELATIVE [VOLUME] [trace]"
                .into(),
        );
    }
    let seed = args
        .iter()
        .find_map(|arg| arg.strip_prefix("seed="))
        .map(str::parse)
        .transpose()?
        .unwrap_or(1);
    let mut scene = scenarios::scene(&args[0], seed, Some(args[1].parse()?))?;
    if let Some(clock) = args.iter().find_map(|arg| arg.strip_prefix("clock=")) {
        scene.clock_drift_ppm = clock.parse()?;
    }
    if let Some(volume) = args.get(5) {
        scene.volume = volume.parse()?;
    }
    let mono = args[2] == "mono";
    let config = EngineConfig {
        floor_calibration_seconds: args
            .iter()
            .find_map(|arg| arg.strip_prefix("calibration="))
            .map(str::parse)
            .transpose()?
            .unwrap_or(0.15),
        f_hi: args
            .iter()
            .find_map(|arg| arg.strip_prefix("high="))
            .map(str::parse)
            .transpose()?
            .unwrap_or(20000.0),
        infer_single_path: args.iter().any(|arg| arg == "infer"),
        resample_reference: args.iter().any(|arg| arg == "clock-warp"),
        regularization: args
            .iter()
            .find_map(|arg| arg.strip_prefix("reg="))
            .map(str::parse)
            .transpose()?
            .unwrap_or(0.001),
        window: args
            .iter()
            .find_map(|arg| arg.strip_prefix("window="))
            .map(str::parse)
            .transpose()?
            .unwrap_or(2048),
        hop: args
            .iter()
            .find_map(|arg| arg.strip_prefix("hop="))
            .map(str::parse)
            .transpose()?
            .unwrap_or(256),
        pc_mode: args[0].starts_with("pc_"),
        fast_seconds: args[3].parse()?,
        floor_relative: args[4].parse()?,
        ..EngineConfig::default()
    };
    let rendered = scene::render(&scene)?;
    let mut engine = Engine::new(
        config,
        StreamFormat {
            sample_rate: scene.fs,
            ref_channels: if mono { 1 } else { 2 },
            mic_channels: scene.mics.len() as u8,
        },
    )?;
    engine.set_source_status(veyl_dsp::SourceStatus {
        capture_running: true,
        source: if args[0] == "blocked" {
            veyl_dsp::SoundSource::Capture
        } else if args[0].starts_with("pc_") {
            veyl_dsp::SoundSource::Pc
        } else {
            veyl_dsp::SoundSource::TestSound
        },
        ..veyl_dsp::SourceStatus::default()
    })?;
    let mut events = Vec::with_capacity(32);
    let mut pokes = vec![];
    let mut trace_frame = 0;
    let mut present = 0;
    let mut peak = 0.0_f32;
    let mut distance = (f32::INFINITY, 0.0_f32);
    let mut last_mode = None;
    let mut mode_changes = 0;
    let mut settled_hops = 0;
    for block in &rendered.blocks {
        let samples = rendered.samples(block).unwrap();
        let pcm: Vec<_> = if mono && block.stream == 0 {
            samples
                .as_chunks::<2>()
                .0
                .iter()
                .map(|frame| quantize((frame[0] + frame[1]) * 0.5))
                .collect()
        } else {
            samples.iter().map(|value| quantize(*value)).collect()
        };
        engine.push(BlockIn {
            stream: if block.stream == 0 {
                Stream::Reference
            } else {
                Stream::Microphone
            },
            first_frame: block.first_frame,
            capture_ns: Some(block.capture_ns),
            flags: BlockFlags(block.flags),
            pre_rms_dbfs: block.pre_rms_dbfs,
            pre_peak: block.pre_peak,
            samples: &pcm,
        })?;
        events.clear();
        engine.process(&mut events)?;
        if let Some(fast) = engine.snapshot().echo {
            if last_mode.is_some_and(|mode| mode != fast.mode) {
                mode_changes += 1;
            }
            last_mode = Some(fast.mode);
        }
        for event in &events {
            if let EngineEvent::Poke(poke) = event {
                pokes.push(*poke);
            }
            if let EngineEvent::Features(feature) = event {
                settled_hops += usize::from(!feature.settling && feature.valid);
                let t = feature.frame as f64 / scene.fs as f64;
                if (4.7..5.5).contains(&t) && feature.valid && !feature.settling {
                    peak = peak.max(feature.z_max);
                    if let Some(value) = feature.distance_cm {
                        present += 1;
                        distance.0 = distance.0.min(value);
                        distance.1 = distance.1.max(value);
                    }
                    if args.iter().any(|arg| arg == "trace") && feature.frame >= trace_frame + 960 {
                        println!("{t:.3}: z {:.2}, peak {:?}, distance {:?}, floor {:.2e}, energy {:.2e}, direct {:.2e}", feature.z_max, feature.r_peak_cm, feature.distance_cm, feature.peak_floor, feature.peak_energy, feature.direct_energy);
                        trace_frame = feature.frame;
                    }
                }
            }
        }
    }
    let mut matched = vec![false; rendered.labels.len()];
    for poke in &pokes {
        if let Some((index, _)) = rendered
            .labels
            .iter()
            .enumerate()
            .filter(|(i, label)| !matched[*i] && label.turn_frame.abs_diff(poke.turn_frame) <= 7200)
            .min_by_key(|(_, label)| label.turn_frame.abs_diff(poke.turn_frame))
        {
            matched[index] = true;
        }
    }
    println!("{}: tau {}, mono {}, volume {}, matched {}/{}, detections {}, first-poke peak {:.2}, present {}, distance {:.2}..{:.2}, drift {:.2} ppm", args[0], args[3], mono, scene.volume, matched.iter().filter(|value| **value).count(), matched.len(), pokes.len(), peak, present, distance.0, distance.1, engine.snapshot().drift_ppm);
    println!(
        "settled hops {settled_hops}, mode changes {mode_changes}, final state {:?}",
        engine.snapshot().state
    );
    println!(
        "path mismatch {:?}",
        engine.snapshot().echo.map(|frame| frame.path_mismatch)
    );
    for poke in pokes {
        println!(
            "turn {:.3}, detect {:.3}, depth {:.2}, distance {:.2}",
            poke.turn_frame as f64 / scene.fs as f64,
            poke.detect_frame as f64 / scene.fs as f64,
            poke.depth_cm,
            poke.distance_cm
        );
    }
    Ok(())
}
