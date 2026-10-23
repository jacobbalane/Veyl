use veyl_dsp::{
    config::EngineConfig, BlockFlags, BlockIn, Engine, EngineEvent, EngineState, Stream,
    StreamFormat, Warning,
};
use veyl_sim::{scenarios, scene};

#[test]
fn source_independent_engine_learns_acoustic_echoes_and_reports_finite_motion_and_hand_events() {
    for name in ["still", "poke_basic", "drift_200ppm", "pc_mode"] {
        let scene = scenarios::scene(name, 1, Some(8.0)).unwrap();
        let rendered = scene::render(&scene).unwrap();
        let config = EngineConfig {
            pc_mode: name.starts_with("pc_"),
            ..EngineConfig::default()
        };
        let format = StreamFormat {
            sample_rate: scene.fs,
            ref_channels: 2,
            mic_channels: scene.mics.len() as u8,
        };
        let mut engine = Engine::new(config, format).unwrap();
        let mut events = Vec::with_capacity(32);
        let mut hops = 0;
        let mut present_hops = 0;
        let mut hand_events = 0;
        let mut jumps = 0;
        let mut peak_z = 0.0_f32;
        let mut min_distance = f32::INFINITY;
        let mut max_distance = 0.0_f32;
        let mut peak_row = 0.0_f32;
        let mut last_state = EngineState::NoAudio;
        for block in &rendered.blocks {
            let pcm: Vec<_> = rendered
                .samples(block)
                .unwrap()
                .iter()
                .map(|sample| (sample * 32768.0).round().clamp(-32768.0, 32767.0) as i16)
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
                    EngineEvent::Features(feature) => {
                        hops += 1;
                        assert!(
                            feature.z_max.is_finite()
                                && feature.mean_energy.is_finite()
                                && feature.peakiness.is_finite()
                        );
                        assert!(feature.distance_cm.is_none_or(f32::is_finite));
                        if feature.valid
                            && !feature.settling
                            && (4.7..5.4).contains(&(feature.frame as f64 / scene.fs as f64))
                        {
                            peak_z = peak_z.max(feature.z_max);
                            if feature.present {
                                present_hops += 1;
                                let distance = feature.distance_cm.unwrap();
                                min_distance = min_distance.min(distance);
                                max_distance = max_distance.max(distance);
                            }
                        }
                    }
                    EngineEvent::Hand { .. } => hand_events += 1,
                    EngineEvent::Warning(Warning::ModelJump) => jumps += 1,
                    _ => {}
                }
            }
            let snapshot = engine.snapshot();
            peak_row = snapshot.motion.iter().copied().fold(peak_row, f32::max);
            last_state = snapshot.state;
        }
        eprintln!("{name}: hops {hops}, hand events {hand_events}, poke-window present hops {present_hops}, peak z {peak_z:.2}, distance {min_distance:.2}..{max_distance:.2}, motion {peak_row:.2}, jumps {jumps}");
        assert!(hops > 800);
        assert_eq!(last_state, EngineState::Running);
        assert_eq!(jumps, 0);
        if name == "poke_basic" {
            assert!(
                peak_z >= 8.0 && present_hops > 20,
                "No concentrated acoustic hand return for {name}."
            );
            assert!(peak_row > 10.0);
        }
        engine.reset();
        assert_eq!(engine.snapshot().state, EngineState::NoAudio);
        assert!(engine.snapshot().hand.distance_cm.is_none());
    }
}
