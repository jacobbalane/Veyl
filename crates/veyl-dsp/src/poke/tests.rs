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
        let mut detector = PokeDetector::new(EngineConfig::specified(), rate).unwrap();
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
        let mut detector = PokeDetector::new(EngineConfig::specified(), 48000).unwrap();
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
fn presence_hysteresis_cannot_confirm_a_turn_whose_motion_is_below_the_on_threshold() {
    for (preset, threshold) in [
        (Sensitivity::Low, 12.0),
        (Sensitivity::Normal, 8.0),
        (Sensitivity::High, 5.0),
    ] {
        let mut detector = PokeDetector::new(EngineConfig::specified(), 48000).unwrap();
        detector.set_sensitivity(preset);
        for frame in (0..48000).step_by(256) {
            let seconds = frame as f32 / 48000.0;
            let mut sample = feature(frame, distance(seconds), true);
            // Presence can remain true for six hops at the lower hysteresis threshold.
            if (0.24..0.30).contains(&seconds) {
                sample.z_max = threshold * 0.75;
            }
            assert!(detector.process(sample, true).is_none());
        }
    }
}

#[test]
fn a_jump_between_echoes_breaks_the_trajectory_and_a_later_real_poke_recovers() {
    let mut detector = PokeDetector::new(EngineConfig::specified(), 48000).unwrap();
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

#[test]
fn a_slow_drift_into_position_does_not_dilute_a_fast_approach() {
    // 4 cm/s for 0.6 s, then the basic poke. From the farthest point the average approach
    // speed is below 10 cm/s; the poke's own approach is 33 cm/s.
    let track = |seconds: f32| {
        if seconds < 0.6 {
            22.4 - 4.0 * seconds
        } else {
            distance(seconds - 0.5)
        }
    };
    let mut detector = PokeDetector::new(EngineConfig::specified(), 48000).unwrap();
    let events: Vec<_> = (0..48000_u64)
        .step_by(256)
        .filter_map(|frame| {
            detector.process(feature(frame, track(frame as f32 / 48000.0), true), true)
        })
        .collect();
    assert_eq!(events.len(), 1);
    assert!((events[0].turn_frame as f32 / 48000.0 - 0.78).abs() < 0.01);
    // The deepest approach that still averages 10 cm/s starts 0.07 s into the drift: 7.1 cm,
    // where the whole drift would have given 7.4 cm at 9.5 cm/s and no poke.
    assert!(
        (7.0..=7.3).contains(&events[0].depth_cm),
        "{}",
        events[0].depth_cm
    );
}

#[test]
fn a_gap_defers_a_poke_instead_of_erasing_it_and_reports_it_when_the_deferral_ends() {
    let mut detector = PokeDetector::new(EngineConfig::specified(), 48000).unwrap();
    let mut events = vec![];
    for frame in (0..48000_u64).step_by(256) {
        let seconds = frame as f32 / 48000.0;
        // A short gap during the retreat: invalid hops with no distance, and no poke may be
        // reported until 300 ms after it.
        let gap = (0.30..0.33).contains(&seconds);
        if gap {
            detector.defer_until(frame + 14400);
        }
        if let Some(event) = detector.process(feature(frame, distance(seconds), !gap), true) {
            events.push(event);
        }
    }
    assert_eq!(events.len(), 1);
    let event = events[0];
    assert!((event.turn_frame as f32 / 48000.0 - 0.28).abs() < 0.01);
    assert!(event.detect_frame as f32 / 48000.0 >= 0.62);
    // Leaving the deferral to a pause still clears everything.
    let mut paused = PokeDetector::new(EngineConfig::specified(), 48000).unwrap();
    paused.defer_until(48000);
    for frame in (0..24000_u64).step_by(256) {
        assert!(paused
            .process(feature(frame, distance(frame as f32 / 48000.0), true), true)
            .is_none());
    }
    assert!(paused.process(feature(48000, 20.0, true), false).is_none());
    assert!(paused.process(feature(48256, 20.0, true), true).is_none());
}
