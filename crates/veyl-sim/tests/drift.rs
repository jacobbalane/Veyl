use veyl_dsp::{
    align::Aligner, config::EngineConfig, echo::EchoEstimator, ring::SampleRing, StreamFormat,
};
use veyl_sim::{scenarios, scene};

#[test]
fn fast_acoustic_maps_drive_continuous_clock_compensation_without_truth_labels() {
    for seed in 1..=3 {
        for name in [
            "drift_50ppm",
            "drift_200ppm",
            "pc_mode",
            "pc_bluetooth",
            "negative_200ppm",
        ] {
            let mut scene = scenarios::scene(
                if name == "negative_200ppm" {
                    "drift_200ppm"
                } else {
                    name
                },
                seed,
                Some(if seed == 1 { 16.0 } else { 8.0 }),
            )
            .unwrap();
            if name == "negative_200ppm" {
                scene.clock_drift_ppm = -200.0;
            }
            let rendered = scene::render(&scene).unwrap();
            let config = EngineConfig::default();
            let format = StreamFormat {
                sample_rate: scene.fs,
                ref_channels: 2,
                mic_channels: scene.mics.len() as u8,
            };
            let mut aligner =
                Aligner::new(config.clone(), format, name.starts_with("pc_")).unwrap();
            let mut reference = SampleRing::new(scene.fs, 2, 4).unwrap();
            let mut microphone = SampleRing::new(scene.fs, format.mic_channels, 4).unwrap();
            let mut estimator = EchoEstimator::new(config.clone(), format).unwrap();
            let mut reference_window = vec![0.0; config.window * 2];
            let mut microphone_window = vec![0.0; config.window * scene.mics.len()];
            let mut next_hop = None;
            let mut lock_frame = None;
            let mut maximum_residual = 0.0_f64;
            let mut samples = 0;
            let mut mode_changes = 0;
            let mut relocks = 0;
            for pair in rendered.blocks.chunks(2) {
                for block in pair {
                    let pcm: Vec<_> = rendered
                        .samples(block)
                        .unwrap()
                        .iter()
                        .map(|sample| (sample * 32768.0).round().clamp(-32768.0, 32767.0) as i16)
                        .collect();
                    let ring = if block.stream == 0 {
                        &mut reference
                    } else {
                        &mut microphone
                    };
                    ring.push(
                        block.first_frame,
                        &pcm,
                        Some(block.capture_ns),
                        block.flags & 1 != 0,
                    )
                    .unwrap();
                }
                if aligner.try_align(&reference, &microphone).is_some() {
                    if lock_frame.is_some() {
                        relocks += 1;
                    }
                    lock_frame.get_or_insert(microphone.end().unwrap());
                    next_hop = Some(microphone.end().unwrap() - 1);
                    estimator.reset();
                }
                while let Some(frame) = next_hop.filter(|frame| *frame < microphone.end().unwrap())
                {
                    let (lag, fraction) = aligner.begin_hop(frame).unwrap();
                    let mic_first = frame + 1 - config.window as u64;
                    let ref_first = u64::try_from(mic_first as i128 - lag as i128).unwrap();
                    for channel in 0..2 {
                        assert!(reference.copy(
                            ref_first,
                            Some(channel),
                            &mut reference_window
                                [channel * config.window..(channel + 1) * config.window]
                        ));
                    }
                    for channel in 0..scene.mics.len() {
                        assert!(microphone.copy(
                            mic_first,
                            Some(channel),
                            &mut microphone_window
                                [channel * config.window..(channel + 1) * config.window]
                        ));
                    }
                    let fast = estimator
                        .process(&reference_window, &microphone_window, fraction, None, false)
                        .unwrap();
                    mode_changes += usize::from(fast.mode_changed);
                    aligner.observe_fast(fast.direct, fast.mode_changed);
                    if frame >= scene.fs as u64 * 5 {
                        maximum_residual =
                            maximum_residual.max(aligner.direct_residual().unwrap().abs());
                        samples += 1;
                    }
                    next_hop = Some(frame + config.hop as u64);
                }
            }
            let ppm = aligner.drift_ppm();
            let details = format!("{name} seed {seed}: residual {maximum_residual:.4}, ppm {ppm:.2}, mode changes {mode_changes}, relocks {relocks}");
            eprintln!("{details}");
            assert!(
                lock_frame.is_some_and(|frame| frame <= scene.fs as u64 * 3),
                "{details}"
            );
            assert!(samples > 500, "{details}");
            assert!(maximum_residual <= 0.3, "{details}");
            assert_eq!(relocks, 0, "{details}");
            assert!((ppm + scene.clock_drift_ppm).abs() < 20.0, "{details}");
        }
    }
}
