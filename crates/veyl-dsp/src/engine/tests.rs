use super::*;

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
        for lag in [-128, 112] {
            let config = EngineConfig::default();
            let mut engine = Engine::new(
                config,
                StreamFormat {
                    sample_rate: rate,
                    ref_channels: 1,
                    mic_channels: 1,
                },
            )
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
        EngineConfig::default(),
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
