use super::*;
fn tracker() -> MotionTracker {
    MotionTracker::new(
        EngineConfig::default(),
        StreamFormat {
            sample_rate: 48000,
            ref_channels: 1,
            mic_channels: 1,
        },
    )
    .unwrap()
}
fn process(
    tracker: &mut MotionTracker,
    frame: u64,
    maps: &[Complex<f32>],
    settling: bool,
    held: bool,
) -> MotionFeatures {
    tracker
        .process(MotionInput {
            frame,
            dynamic: maps,
            normalized: None,
            direct_position: 0.0,
            direct_energy: 1.0,
            reference_weights: [1.0, 0.0],
            reference_cross: Complex::default(),
            settling,
            held,
            energy_floor: false,
        })
        .unwrap()
}
#[test]
fn motion_requires_presence_hysteresis_freezes_its_floor_and_rejects_distance_outliers() {
    let mut tracker = tracker();
    let taps = tracker.taps;
    let baseline = vec![Complex::new(0.001, 0.0); taps];
    for hop in 0..30 {
        assert!(!process(&mut tracker, hop * 256, &baseline, true, false).present);
    }
    let k = tracker.config.pre_taps + 60;
    let mut moving = baseline.clone();
    moving[k] = Complex::new(0.02, 0.0);
    for hop in 30..32 {
        assert!(!process(&mut tracker, hop * 256, &moving, false, false).present);
    }
    let onset = process(&mut tracker, 32 * 256, &moving, false, false);
    assert!(onset.present && onset.changed);
    assert!((onset.distance_cm.unwrap() - 21.46).abs() < 0.1);
    let floor = tracker.floor()[k];
    for hop in 33..60 {
        assert!(process(&mut tracker, hop * 256, &moving, false, false).present);
    }
    assert_eq!(tracker.floor()[k], floor);
    let mut outlier = baseline.clone();
    outlier[tracker.config.pre_taps + 100] = Complex::new(0.02, 0.0);
    let distance = tracker.features().distance_cm.unwrap();
    assert!(
        (process(&mut tracker, 60 * 256, &outlier, false, false)
            .distance_cm
            .unwrap()
            - distance)
            .abs()
            < 0.01
    );
    let held = process(&mut tracker, 61 * 256, &baseline, false, true);
    assert!(!held.valid && held.present && !held.changed);
    for hop in 62..67 {
        assert!(process(&mut tracker, hop * 256, &baseline, false, false).present);
    }
    let end = process(&mut tracker, 67 * 256, &baseline, false, false);
    assert!(!end.present && end.changed && end.distance_cm.is_none());
    let row = tracker.take_row();
    assert!(row[21] > 20.0);
    assert!(tracker.row().iter().all(|value| *value == 0.0));
}

#[test]
fn range_presets_and_channel_power_weights_apply_before_presence() {
    let mut tracker = tracker();
    let mut maps = vec![Complex::new(0.001, 0.0); tracker.taps];
    process(&mut tracker, 0, &maps, true, false);
    let far = tracker.config.pre_taps + 140;
    maps[far] = Complex::new(0.1, 0.0);
    for hop in 1..8 {
        assert!(!process(&mut tracker, hop * 256, &maps, false, false).present);
    }
    tracker.set_range(RangePreset::Far);
    for hop in 8..10 {
        assert!(!process(&mut tracker, hop * 256, &maps, false, false).present);
    }
    assert!(process(&mut tracker, 10 * 256, &maps, false, false).present);
    assert_eq!(tracker.take_row()[50], 30.0);
    tracker.set_sensitivity(Sensitivity::Low);
    assert!(!tracker.present());
    assert_eq!(tracker.z_on, 12.0);
    tracker.reset();
    assert!(tracker.floor().iter().all(|value| *value == 0.0));
    let config = EngineConfig::default();
    let mut stereo = MotionTracker::new(
        config.clone(),
        StreamFormat {
            sample_rate: 48000,
            ref_channels: 2,
            mic_channels: 1,
        },
    )
    .unwrap();
    let mut pairs = vec![Complex::default(); stereo.taps * 2];
    let k = config.pre_taps + 50;
    pairs[k] = Complex::new(0.02, 0.0);
    pairs[stereo.taps + k] = Complex::new(0.04, 0.0);
    stereo
        .process(MotionInput {
            frame: 0,
            dynamic: &pairs,
            normalized: None,
            direct_position: 0.0,
            direct_energy: 1.0,
            reference_weights: [0.25, 0.75],
            reference_cross: Complex::default(),
            settling: false,
            held: false,
            energy_floor: false,
        })
        .unwrap();
    assert!((stereo.normalized_motion()[k] * stereo.floor()[k] - 0.0013).abs() < 1e-8);
}

#[test]
fn calibration_measures_model_variance_instead_of_small_adjacent_hop_differences() {
    let mut tracker = tracker();
    let k = tracker.config.pre_taps + 60;
    let mut samples = vec![Complex::default(); tracker.taps];
    let mut changes = samples.clone();
    let mut previous = 0.0;
    for hop in 0..600 {
        let value = 0.01 * (std::f32::consts::TAU * hop as f32 * 256.0 / 48000.0 / 0.5).sin();
        samples[k] = Complex::new(0.1 + value, 0.0);
        changes[k] = Complex::new(value - previous, 0.0);
        previous = value;
        tracker
            .process(MotionInput {
                frame: hop * 256,
                dynamic: &changes,
                normalized: Some(&samples),
                direct_position: 0.0,
                direct_energy: 1.0,
                reference_weights: [1.0, 0.0],
                reference_cross: Complex::default(),
                settling: true,
                held: false,
                energy_floor: false,
            })
            .unwrap();
    }
    assert!((2e-5..8e-5).contains(&tracker.floor()[k]));
    for hop in 600..900 {
        let value = 0.01 * (std::f32::consts::TAU * hop as f32 * 256.0 / 48000.0 / 0.5).sin();
        changes[k] = Complex::new(value, 0.0);
        assert!(!process(&mut tracker, hop * 256, &changes, false, false).present);
    }
}

#[test]
fn a_disappeared_background_echo_cannot_hide_a_new_moving_return() {
    let mut tracker = tracker();
    let mut dynamic = vec![Complex::default(); tracker.taps];
    let mut current = dynamic.clone();
    let old = tracker.config.pre_taps + 60;
    let new = tracker.config.pre_taps + 45;
    dynamic[old] = Complex::new(-0.1, 0.0);
    dynamic[new] = Complex::new(0.02, 0.0);
    current[new] = Complex::new(0.02, 0.0);
    for hop in 0..3 {
        tracker
            .process(MotionInput {
                frame: hop * 256,
                dynamic: &dynamic,
                normalized: Some(&current),
                direct_position: 0.0,
                direct_energy: 1.0,
                reference_weights: [1.0, 0.0],
                reference_cross: Complex::default(),
                settling: false,
                held: false,
                energy_floor: false,
            })
            .unwrap();
    }
    assert!(tracker.present());
    assert_eq!(tracker.normalized_motion()[old], 0.0);
    assert!((tracker.features().distance_cm.unwrap() - 45.0 * tracker.cm_per_tap).abs() < 0.01);
}

#[test]
fn correlated_references_cancel_unobservable_opposite_transfer_changes() {
    let config = EngineConfig::default();
    let mut tracker = MotionTracker::new(
        config.clone(),
        StreamFormat {
            sample_rate: 48000,
            ref_channels: 2,
            mic_channels: 1,
        },
    )
    .unwrap();
    let mut maps = vec![Complex::default(); tracker.taps * 2];
    let k = config.pre_taps + 50;
    maps[k] = Complex::new(0.02, 0.0);
    maps[tracker.taps + k] = Complex::new(-0.02, 0.0);
    for (hop, correlation) in [0.5, 0.0].into_iter().enumerate() {
        tracker
            .process(MotionInput {
                frame: hop as u64 * 256,
                dynamic: &maps,
                normalized: None,
                direct_position: 0.0,
                direct_energy: 1.0,
                reference_weights: [0.5, 0.5],
                reference_cross: Complex::new(correlation, 0.0),
                settling: false,
                held: false,
                energy_floor: false,
            })
            .unwrap();
        if hop == 0 {
            assert!(tracker.normalized_motion()[k] < 1e-5);
        } else {
            assert!(tracker.normalized_motion()[k] > 100.0);
        }
    }
}

#[test]
fn a_present_hand_keeps_its_track_through_a_stronger_burst_elsewhere() {
    for gate in [0.0, 4.0] {
        let mut tracker = MotionTracker::new(
            EngineConfig {
                track_gate_cm: gate,
                ..EngineConfig::default()
            },
            StreamFormat {
                sample_rate: 48000,
                ref_channels: 1,
                mic_channels: 1,
            },
        )
        .unwrap();
        let (taps, pre) = (tracker.taps, tracker.config.pre_taps);
        let baseline = vec![Complex::new(0.001, 0.0); taps];
        for hop in 0..30 {
            process(&mut tracker, hop * 256, &baseline, true, false);
        }
        let mut hand = baseline.clone();
        hand[pre + 60] = Complex::new(0.02, 0.0);
        for hop in 30..40 {
            let present = process(&mut tracker, hop * 256, &hand, false, false).present;
            assert_eq!(present, hop >= 32);
        }
        // A brief, stronger burst 14 cm farther away, such as a transient at a static echo.
        let mut burst = hand.clone();
        burst[pre + 100] = Complex::new(0.05, 0.0);
        let peak = process(&mut tracker, 40 * 256, &burst, false, false)
            .r_peak_cm
            .unwrap();
        if gate > 0.0 {
            assert!((peak - 21.46).abs() < 0.5, "gated peak at {peak} cm");
        } else {
            assert!((peak - 35.77).abs() < 0.5, "ungated peak at {peak} cm");
        }
    }
}

#[test]
fn a_farther_microphone_direct_sound_is_left_out_of_its_motion() {
    let format = StreamFormat {
        sample_rate: 48000,
        ref_channels: 1,
        mic_channels: 2,
    };
    let mut tracker = MotionTracker::new(EngineConfig::default(), format).unwrap();
    let (taps, pre) = (tracker.taps, tracker.config.pre_taps);
    // The second microphone's direct sound arrives 30 taps after the nearest one's.
    tracker.set_own_directs(&[Some(0.0), Some(30.0)]);
    let mut maps = vec![Complex::new(0.001, 0.0); 2 * taps];
    for hop in 0..30 {
        tracker
            .process(MotionInput {
                frame: hop * 256,
                dynamic: &maps,
                normalized: None,
                direct_position: 0.0,
                direct_energy: 1.0,
                reference_weights: [1.0, 0.0],
                reference_cross: Complex::default(),
                settling: true,
                held: false,
                energy_floor: false,
            })
            .unwrap();
    }
    // Model error around the farther microphone's own direct sound, and a hand farther out.
    maps[taps + pre + 32] = Complex::new(0.05, 0.0);
    maps[pre + 60] = Complex::new(0.02, 0.0);
    let feature = tracker
        .process(MotionInput {
            frame: 30 * 256,
            dynamic: &maps,
            normalized: None,
            direct_position: 0.0,
            direct_energy: 1.0,
            reference_weights: [1.0, 0.0],
            reference_cross: Complex::default(),
            settling: false,
            held: false,
            energy_floor: false,
        })
        .unwrap();
    assert!((feature.r_peak_cm.unwrap() - 21.46).abs() < 0.5);
    // Only the nearer microphone's unchanged baseline remains at that tap.
    assert!((tracker.normalized_motion()[pre + 32] - 1.0).abs() < 1e-3);
}
