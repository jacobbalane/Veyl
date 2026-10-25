use super::*;

#[test]
fn source_control_changes_clear_pending_pokes_but_media_volume_updates_preserve_them() {
    for change_source in [false, true] {
        let mut engine = Engine::new(
            EngineConfig::specified(),
            StreamFormat {
                sample_rate: 48000,
                ref_channels: 1,
                mic_channels: 1,
            },
        )
        .unwrap();
        let mut status = SourceStatus {
            capture_running: true,
            source: crate::SoundSource::TestSound,
            media_volume: Some(0.4),
            ..SourceStatus::default()
        };
        engine.set_source_status(status).unwrap();
        let mut updated = false;
        let mut pokes = 0;
        for frame in (0..48000).step_by(256) {
            let t = frame as f32 / 48000.0;
            let distance = if t < 0.1 {
                20.0
            } else if t < 0.25 {
                20.0 - 5.0 * (t - 0.1) / 0.15
            } else if t < 0.28 {
                15.0
            } else if t < 0.48 {
                15.0 + 5.0 * (t - 0.28) / 0.2
            } else {
                20.0
            };
            if !updated && t >= 0.2 {
                if change_source {
                    status.source = crate::SoundSource::Player;
                } else {
                    status.media_volume = Some(0.8);
                }
                engine.set_source_status(status).unwrap();
                updated = true;
            }
            pokes += usize::from(
                engine
                    .detector
                    .process(
                        MotionFeatures {
                            frame,
                            valid: true,
                            present: true,
                            distance_cm: Some(distance),
                            z_max: 20.0,
                            ..MotionFeatures::default()
                        },
                        true,
                    )
                    .is_some(),
            );
        }
        assert_eq!(pokes, usize::from(!change_source));
    }
}

#[test]
fn signed_lags_wait_for_matching_reference_and_gaps_hold_instead_of_inventing_motion() {
    for rate in [44100, 48000] {
        let frames = rate as usize * 5;
        let mut state = 47_u64;
        let reference: Vec<_> = (0..frames)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                ((state >> 40) as f32 / 16777216.0 * 10000.0 - 5000.0) as i16
            })
            .collect();
        for (lag, resample_reference) in [(-128, false), (112, false), (-128, true), (112, true)] {
            let config = EngineConfig {
                resample_reference,
                ..EngineConfig::specified()
            };
            let mut engine = Engine::new(
                config,
                StreamFormat {
                    sample_rate: rate,
                    ref_channels: 1,
                    mic_channels: 1,
                },
            )
            .unwrap();
            engine
                .set_source_status(SourceStatus {
                    capture_running: true,
                    ..SourceStatus::default()
                })
                .unwrap();
            let block = rate as usize / 100;
            let mut events = Vec::with_capacity(32);
            for first in (0..frames - block).step_by(block) {
                if first / block == 420 {
                    continue;
                }
                let microphone: Vec<_> = (first..first + block)
                    .map(|index| {
                        usize::try_from(index as i64 - lag)
                            .ok()
                            .and_then(|at| reference.get(at))
                            .copied()
                            .unwrap_or(0)
                    })
                    .collect();
                for (stream, samples) in [
                    (Stream::Reference, &reference[first..first + block]),
                    (Stream::Microphone, &microphone[..]),
                ] {
                    engine
                        .push(BlockIn {
                            stream,
                            first_frame: first as u64,
                            capture_ns: Some(first as i64 * 1_000_000_000 / rate as i64),
                            flags: BlockFlags(0),
                            pre_rms_dbfs: -20.0,
                            pre_peak: 0.2,
                            samples,
                        })
                        .unwrap();
                    events.clear();
                    engine.process(&mut events).unwrap();
                }
            }
            let snapshot = engine.snapshot();
            assert_eq!(snapshot.state, EngineState::Running);
            assert!((snapshot.lag_frames.unwrap() - lag).abs() <= 1);
            assert!(snapshot.invalid_hops > 0 && snapshot.processed_hops > 400);
            engine.set_paused(true);
            assert!(engine.snapshot().suspended);
            engine.set_paused(false);
            assert!(!engine.snapshot().suspended);
        }
    }
}

#[test]
fn silent_input_waits_and_malformed_blocks_do_not_create_an_alignment() {
    let mut engine = Engine::new(
        EngineConfig::specified(),
        StreamFormat {
            sample_rate: 48000,
            ref_channels: 1,
            mic_channels: 1,
        },
    )
    .unwrap();
    let mut events = Vec::with_capacity(32);
    engine.process(&mut events).unwrap();
    assert_eq!(engine.snapshot().state, EngineState::NoAudio);
    let silence = [0; 480];
    for first in (0..144000).step_by(480) {
        for stream in [Stream::Reference, Stream::Microphone] {
            engine
                .push(BlockIn {
                    stream,
                    first_frame: first,
                    capture_ns: None,
                    flags: BlockFlags(0),
                    pre_rms_dbfs: -120.0,
                    pre_peak: 0.0,
                    samples: &silence,
                })
                .unwrap();
        }
        events.clear();
        engine.process(&mut events).unwrap();
        assert!(!events.iter().any(|event| matches!(
            event,
            EngineEvent::Aligned { .. } | EngineEvent::Hand { present: true, .. }
        )));
    }
    assert_eq!(engine.snapshot().state, EngineState::WaitingForSound);
    assert!(engine
        .push(BlockIn {
            stream: Stream::Reference,
            first_frame: 144000,
            capture_ns: None,
            flags: BlockFlags(0),
            pre_rms_dbfs: -120.0,
            pre_peak: 0.0,
            samples: &[0; 479]
        })
        .is_err());
}
