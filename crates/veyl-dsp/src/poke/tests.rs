use super::*;
fn distance(seconds: f32) -> f32 {
    if seconds < 0.1 {
        20.0
    } else if seconds < 0.25 {
        20.0 - 5.0 * (seconds - 0.1) / 0.15
    } else if seconds < 0.28 {
        15.0
    } else if seconds < 0.48 {
        15.0 + 5.0 * (seconds - 0.28) / 0.2
    } else {
        20.0
    }
}
fn feature(frame: u64, distance: f32, valid: bool) -> MotionFeatures {
    MotionFeatures {
        frame,
        valid,
        present: true,
        distance_cm: Some(distance),
        z_max: 20.0,
        ..MotionFeatures::default()
    }
}
#[test]
fn approach_turn_retreat_fires_once_with_frame_timing_and_confidence_at_both_rates() {
    for rate in [44100, 48000] {
        let mut detector = PokeDetector::new(EngineConfig::default(), rate).unwrap();
        let mut events = vec![];
        for frame in (0..rate as u64).step_by(256) {
            if let Some(event) = detector.process(
                feature(frame, distance(frame as f32 / rate as f32), true),
                true,
            ) {
                events.push(event);
            }
        }
        assert_eq!(events.len(), 1);
        let event = events[0];
        assert!((event.turn_frame as f32 / rate as f32 - 0.28).abs() < 0.01);
        assert!((0.35..0.38).contains(&(event.detect_frame as f32 / rate as f32)));
        assert!((event.depth_cm - 5.0).abs() < 0.1);
        assert!((0.8..=0.84).contains(&event.confidence));
    }
}
#[test]
fn slow_shallow_invalid_and_suspended_motion_cannot_fire_and_presets_reset_history() {
    for mode in 0..4 {
        let mut detector = PokeDetector::new(EngineConfig::default(), 48000).unwrap();
        for (hop, frame) in (0..48000).step_by(256).enumerate() {
            let seconds = frame as f32 / 48000.0;
            let distance = match mode {
                0 => distance(seconds * 0.2),
                1 => 20.0 - (20.0 - distance(seconds)) * 0.3,
                _ => distance(seconds),
            };
            assert!(detector
                .process(
                    feature(frame, distance, mode != 2 || hop % 3 != 0),
                    mode != 3 || seconds > 0.3
                )
                .is_none());
        }
        detector.set_sensitivity(Sensitivity::High);
        assert_eq!(detector.thresholds, (2.0, 1.5, 8.0));
        detector.set_range(RangePreset::Near);
        assert_eq!(detector.range, (4.0, 25.0));
        detector.reset();
        assert!(detector.history.is_empty());
    }
}

#[test]
fn a_jump_between_echoes_breaks_the_trajectory_and_a_later_real_poke_recovers() {
    let mut detector = PokeDetector::new(EngineConfig::default(), 48000).unwrap();
    let mut pokes = vec![];
    for frame in (0..96000).step_by(256) {
        let t = frame as f32 / 48000.0;
        let r = if t < 0.2 {
            35.0
        } else if t < 0.4 {
            10.0
        } else if t < 0.7 {
            35.0
        } else {
            distance(t - 1.0)
        };
        if let Some(event) = detector.process(feature(frame, r, true), true) {
            pokes.push(event);
        }
    }
    assert_eq!(pokes.len(), 1);
    assert!(pokes[0].turn_frame > 48000);
}
