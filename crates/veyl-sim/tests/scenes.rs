use veyl_sim::{
    scenarios,
    scene::{self, *},
    SCENARIOS,
};
fn power(signal: &[f32]) -> f64 {
    signal
        .iter()
        .map(|&sample| (sample as f64).powi(2))
        .sum::<f64>()
        / signal.len().max(1) as f64
}

#[test]
fn microphone_clock_conversion_does_not_modulate_a_stationary_high_band_return() {
    for ppm in [-200.0, 200.0] {
        let mut scene = Scene::phone(1, 1.0);
        scene.clock_drift_ppm = ppm;
        scene.volume = 0.05;
        scene.mic_noise_dbfs = -160.0;
        scene.mics.truncate(1);
        scene.statics.clear();
        scene.music = Music::Samples(
            (0..48000)
                .map(|i| {
                    let value =
                        (std::f64::consts::TAU * 20000.0 * i as f64 / 48000.0).sin() as f32 * 0.4;
                    [value; 2]
                })
                .collect(),
        );
        let rendered = scene::render(&scene).unwrap();
        let frequency = std::f64::consts::TAU * 20000.0 * (1.0 + ppm * 1e-6) / 48000.0;
        let mut minimum = f64::INFINITY;
        let mut maximum = 0.0_f64;
        for start in (12000..45000).step_by(240) {
            let (mut cc, mut ss, mut cs, mut xc, mut xs) = (0.0, 0.0, 0.0, 0.0, 0.0);
            for i in start..start + 240 {
                let (sin, cos) = (frequency * i as f64).sin_cos();
                let value = rendered.mic[i] as f64;
                cc += cos * cos;
                ss += sin * sin;
                cs += cos * sin;
                xc += value * cos;
                xs += value * sin;
            }
            let determinant = cc * ss - cs * cs;
            let amplitude =
                ((xc * ss - xs * cs) / determinant).hypot((xs * cc - xc * cs) / determinant);
            minimum = minimum.min(amplitude);
            maximum = maximum.max(amplitude);
        }
        let ripple = 20.0 * (maximum / minimum).log10();
        eprintln!("clock {ppm} ppm: stationary 20 kHz amplitude ripple {ripple:.6} dB");
        assert!(
            ripple < 0.02,
            "Clock conversion changed a stationary return's amplitude."
        );
        assert!(rendered.labels.is_empty());
    }
}
#[test]
fn acoustic_delay_and_clock_scenarios_lock_from_pcm_and_capture_timestamps_within_three_seconds() {
    use veyl_dsp::{
        align::{AlignmentLock, GccPhat},
        config::EngineConfig,
    };
    for seed in 1..=3 {
        for name in [
            "latency_200ms",
            "drift_50ppm",
            "drift_200ppm",
            "pc_bluetooth",
        ] {
            let scene = scenarios::scene(name, seed, Some(3.0)).unwrap();
            let rendered = scene::render(&scene).unwrap();
            let mono = |samples: &[f32], channels| {
                samples
                    .chunks_exact(channels)
                    .map(|frame| frame.iter().sum::<f32>() / channels as f32)
                    .collect::<Vec<_>>()
            };
            let reference = mono(&rendered.reference, 2);
            let mic = mono(&rendered.mic, scene.mics.len());
            let config = EngineConfig::specified();
            let mut gcc = GccPhat::new(48000, &config, 24000).unwrap();
            let mut lock = AlignmentLock::new(48000, config).unwrap();
            for first in [0, 48000] {
                let at = first as u64 + 24000;
                let ns = at * 1_000_000_000 / 48000;
                let anchor = rendered
                    .blocks
                    .iter()
                    .filter(|block| block.stream == 1)
                    .min_by_key(|block| block.capture_ns.abs_diff(ns as i64))
                    .unwrap();
                let timestamp_offset = anchor.first_frame as f64
                    + (ns as i64 - anchor.capture_ns) as f64 * 48000.0 / 1e9
                    - at as f64;
                let estimate = gcc
                    .estimate_near(
                        &reference[first..first + 48000],
                        &mic[first..first + 72000],
                        -1440..=24000,
                        lock.expected_lag(timestamp_offset, None),
                    )
                    .unwrap();
                lock.observe((first + 72000) as u64, estimate, timestamp_offset, None);
            }
            assert!(lock.locked(), "PCM alignment did not lock for {name}");
        }
    }
}
#[test]
fn every_catalogue_scene_has_finite_filtered_pcm_real_metadata_and_valid_seeded_parameters() {
    for seed in 1..=3 {
        for name in SCENARIOS {
            let scene = scenarios::scene(name, seed, Some(0.12)).unwrap();
            let rendered = scene::render(&scene).unwrap();
            assert_eq!(rendered.reference.len(), 5760 * 2);
            assert_eq!(rendered.mic.len(), 5760 * scene.mics.len());
            assert!(rendered
                .reference
                .iter()
                .chain(&rendered.mic)
                .all(|sample| sample.is_finite() && sample.abs() <= 1.0));
            for block in &rendered.blocks {
                assert_eq!(block.first_frame, block.seq as u64 * 480);
                assert!(block.pre_rms_dbfs.is_finite() && block.pre_peak.is_finite());
                assert_eq!(
                    rendered.samples(block).unwrap().len(),
                    480 * if block.stream == 0 {
                        2
                    } else {
                        scene.mics.len()
                    }
                );
            }
        }
    }
    assert!(scenarios::scene("unknown", 1, None).is_err());
    assert!(scenarios::scene("still", 0, None).is_err());
}
#[test]
fn trajectories_and_labels_follow_geometry_and_keep_clock_drift_in_microphone_frame_time() {
    let scene = scenarios::scene("poke_varied", 3, None).unwrap();
    let Mover::Pokes(pokes) = &scene.movers[0] else {
        panic!("Expected poke trajectory.")
    };
    assert_eq!(pokes.len(), 30);
    for poke in pokes {
        let from = scene.movers[0].position(poke.start());
        let turn = scene.movers[0].position(poke.turn_seconds);
        assert!((from[2] - turn[2] - poke.depth).abs() < 1e-8);
        assert!((0.025..=0.08).contains(&poke.depth));
    }
    let scene = scenarios::scene("pc_mode", 1, Some(5.3)).unwrap();
    let first = scene::render(&scene).unwrap();
    let second = scene::render(&scene).unwrap();
    assert!(first.reference == second.reference && first.mic == second.mic);
    assert_eq!(first.labels.len(), 1);
    assert_eq!(
        first.labels[0].turn_frame,
        (5.0 * 48000.0 / 1.00015_f64).round() as u64
    );
    assert!(first.labels[0].depth_cm > 3.0);
    assert_eq!(scene.speakers.len(), 2);
    assert_eq!(scene.clock_drift_ppm, 150.0);
    assert_eq!(
        scenarios::scene("pc_bluetooth", 1, None)
            .unwrap()
            .output_latency_ms,
        350.0
    );
}
#[test]
fn protected_capture_gaps_and_silence_affect_audio_and_received_blocks_without_fabricated_features()
{
    let blocked = scene::render(&scenarios::scene("blocked", 1, Some(0.5)).unwrap()).unwrap();
    assert!(blocked.reference.iter().all(|&sample| sample == 0.0));
    assert!(power(&blocked.mic) > 1e-7);
    let silent = scene::render(&scenarios::scene("silence", 1, Some(0.5)).unwrap()).unwrap();
    assert!(silent.reference.iter().all(|&sample| sample == 0.0));
    assert!(power(&silent.mic) < 1e-8);
    let mut scene = scenarios::scene("still", 1, Some(0.5)).unwrap();
    scene.dropouts = vec![(0.2, 0.03)];
    let gapped = scene::render(&scene).unwrap();
    assert_eq!(gapped.blocks.len(), 94);
    assert!(gapped
        .blocks
        .iter()
        .all(|block| !(20..23).contains(&block.seq)));
    assert!(gapped
        .blocks
        .iter()
        .filter(|block| block.seq == 23)
        .all(|block| block.flags & 1 == 1));
    let mut invalid = Scene::phone(1, 0.1);
    invalid.clock_drift_ppm = f64::NAN;
    assert!(scene::render(&invalid).is_err());
}
#[test]
fn driver_latency_is_in_the_waveform_and_pre_filter_statistics_precede_privacy_filtering() {
    let mut scene = scenarios::scene("latency_200ms", 2, Some(1.5)).unwrap();
    scene.mics.truncate(1);
    let rendered = scene::render(&scene).unwrap();
    let reference: Vec<_> = rendered
        .reference
        .as_chunks::<2>()
        .0
        .iter()
        .map(|frame| (frame[0] + frame[1]) * 0.5)
        .collect();
    let check = veyl_dsp::sound_check::analyze(&reference, &rendered.mic, 48000).unwrap();
    assert!((check.lag_frames - 9606).abs() <= 2);
    assert!(check.mean_coherence > 0.7);
    let pre = rendered
        .blocks
        .iter()
        .filter(|block| block.stream == 0)
        .map(|block| 10.0_f64.powf(block.pre_rms_dbfs as f64 / 10.0))
        .sum::<f64>()
        / 150.0;
    assert!(pre > power(&rendered.reference) * 10.0);
}
