//! This candidate covers clock drift, not the complete M4 gesture acceptance matrix.
use veyl_dsp::{
    config::EngineConfig, BlockFlags, BlockIn, Engine, EngineEvent, EngineState, SoundSource,
    SourceStatus, Stream, StreamFormat, Warning,
};
use veyl_sim::{basic::quantize, scenarios, scene};

#[test]
fn resampled_reference_recovers_full_phone_clock_scenarios_without_false_pokes_or_drift_suspension()
{
    for (name, seed, reverse) in [
        ("drift_50ppm", 1, false),
        ("drift_50ppm", 2, false),
        ("drift_50ppm", 3, false),
        ("drift_200ppm", 1, false),
        ("drift_200ppm", 2, false),
        ("drift_200ppm", 3, false),
        ("drift_200ppm", 1, true),
    ] {
        let mut scene = scenarios::scene(name, seed, None).unwrap();
        if reverse {
            scene.clock_drift_ppm *= -1.0;
        }
        let rendered = scene::render(&scene).unwrap();
        let mut engine = Engine::new(
            EngineConfig {
                fast_seconds: 0.005,
                infer_single_path: true,
                resample_reference: true,
                ..EngineConfig::default()
            },
            StreamFormat {
                sample_rate: scene.fs,
                ref_channels: 2,
                mic_channels: scene.mics.len() as u8,
            },
        )
        .unwrap();
        engine
            .set_source_status(SourceStatus {
                capture_running: true,
                source: SoundSource::TestSound,
                ..SourceStatus::default()
            })
            .unwrap();
        let mut events = Vec::with_capacity(32);
        let mut pokes = vec![];
        let mut maximum_residual = 0.0_f64;
        let mut residuals = 0;
        let mut relocks = 0;
        let mut drift_warnings = 0;
        for block in &rendered.blocks {
            let pcm: Vec<_> = rendered
                .samples(block)
                .unwrap()
                .iter()
                .map(|v| quantize(*v))
                .collect();
            engine
                .push(BlockIn {
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
                })
                .unwrap();
            events.clear();
            engine.process(&mut events).unwrap();
            for event in &events {
                match event {
                    EngineEvent::Poke(poke) => pokes.push(*poke),
                    EngineEvent::Warning(Warning::Relocked) => relocks += 1,
                    EngineEvent::Warning(Warning::Drift) => drift_warnings += 1,
                    _ => {}
                }
            }
            let snapshot = engine.snapshot();
            if snapshot
                .frame
                .is_some_and(|frame| frame >= scene.fs as u64 * 5)
            {
                if let Some(residual) = snapshot.direct_residual {
                    maximum_residual = maximum_residual.max(residual.abs());
                    residuals += 1;
                }
            }
        }
        eprintln!("{name}, seed {seed}, reverse {reverse}: {} detections, maximum residual {maximum_residual:.6} taps", pokes.len());
        assert_eq!(
            pokes.len(),
            10,
            "Missing or extra pokes for {name}, seed {seed}, reverse {reverse}."
        );
        let mut matched = vec![false; rendered.labels.len()];
        let mut delays = vec![];
        for poke in pokes {
            let index = rendered
                .labels
                .iter()
                .enumerate()
                .filter(|(i, label)| {
                    !matched[*i]
                        && label.turn_frame.abs_diff(poke.turn_frame)
                            <= scene.fs as u64 * 150 / 1000
                })
                .min_by_key(|(_, label)| label.turn_frame.abs_diff(poke.turn_frame))
                .map(|(i, _)| i)
                .expect("A detected poke has no unused truth label.");
            matched[index] = true;
            delays.push(poke.detect_frame as i64 - rendered.labels[index].turn_frame as i64);
        }
        assert!(matched.iter().all(|hit| *hit));
        delays.sort_unstable();
        assert!((delays[4] + delays[5]) as f64 * 0.5 <= scene.fs as f64 * 0.08);
        assert!(maximum_residual <= 0.3 && residuals > 10000);
        assert_eq!(relocks, 0);
        assert_eq!(drift_warnings, 0);
        assert_eq!(engine.snapshot().state, EngineState::Running);
        assert!((engine.snapshot().drift_ppm + scene.clock_drift_ppm).abs() < 1.0);
    }
}
