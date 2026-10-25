use super::*;
fn noise(size: usize, seed: u64) -> Vec<f32> {
    let mut state = seed;
    (0..size)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 40) as f32 / 16777216.0 - 0.5
        })
        .collect()
}
#[test]
fn gcc_phat_finds_signed_lags_with_noise_rejects_silence_and_does_not_lock_unrelated_audio() {
    let config = EngineConfig::specified();
    for rate in [44100, 48000] {
        let reference = noise(rate as usize, 7);
        let unrelated = noise(rate as usize * 2, 21);
        let mut gcc = GccPhat::new(rate, &config, rate as usize).unwrap();
        for lag in [-128, 0, rate as i64 / 5, rate as i64 * 35 / 100] {
            let mic: Vec<_> = (0..rate as usize * 3 / 2)
                .map(|i| {
                    let index = i as i64 - lag;
                    (if index >= 0 {
                        reference.get(index as usize).copied().unwrap_or(0.0)
                    } else {
                        0.0
                    }) * 0.3
                        + unrelated[i] * 0.01
                })
                .collect();
            let estimate = gcc
                .estimate(&reference, &mic, -1000..=rate as i64 / 2)
                .unwrap();
            assert_eq!(estimate.lag_frames, lag);
            assert!(estimate.psr > 6.0);
            assert!((estimate.fractional_lag - lag as f64).abs() < 0.1);
            let wrong_prior = gcc
                .estimate_near(
                    &reference,
                    &mic,
                    -1000..=rate as i64 / 2,
                    Some(lag as f64 + 500.0),
                )
                .unwrap();
            assert_eq!(wrong_prior.lag_frames, lag);
        }
        assert!(gcc
            .estimate(
                &vec![0.0; rate as usize],
                &vec![0.0; rate as usize],
                -1000..=1000
            )
            .is_none());
        let estimate = gcc
            .estimate(&reference, &unrelated[..rate as usize], -1000..=1000)
            .unwrap();
        let mut lock = AlignmentLock::new(rate, config.clone()).unwrap();
        assert!(lock.observe(rate as u64, estimate, 0.0, None).is_none());
        assert!(lock.observe(rate as u64 * 2, estimate, 0.0, None).is_none());
        assert!(!lock.locked());
    }
}
fn estimate(lag: f64) -> LagEstimate {
    LagEstimate {
        lag_frames: lag.round() as i64,
        fractional_lag: lag,
        psr: 30.0,
        reference_band_dbfs: -30.0,
    }
}
#[test]
fn lock_requires_two_spaced_estimates_and_rechecks_ignore_explained_drift_but_relock_a_route_jump()
{
    let mut lock = AlignmentLock::new(48000, EngineConfig::specified()).unwrap();
    assert!(lock.observe(48000, estimate(2000.0), 0.0, None).is_none());
    assert!(lock.observe(48001, estimate(2000.0), 0.0, None).is_none());
    assert!(matches!(
        lock.observe(96000, estimate(1990.4), -9.6, None),
        Some(LockChange::Locked(_))
    ));
    assert!(lock
        .observe(336000, estimate(1942.4), -57.6, Some(1942.4))
        .is_none());
    assert!(lock
        .observe(576000, estimate(2100.0), 0.0, Some(1894.4))
        .is_none());
    assert!(matches!(
        lock.observe(816000, estimate(2052.0), 0.0, Some(1846.4)),
        Some(LockChange::Relocked(_))
    ));
    lock.reset();
    assert!(!lock.locked());
}
#[test]
fn kalman_feedback_tracks_both_clock_signs_and_keeps_delay_continuous_through_integer_folds() {
    for ppm in [-200.0, -50.0, 50.0, 200.0] {
        let config = EngineConfig::specified();
        let mut tracker = DriftTracker::new(&config);
        let mut integer = 0;
        for hop in 1..=48000 * 8 / 256 {
            tracker.predict(256);
            let applied = tracker.fraction();
            let true_delay = hop as f64 * 256.0 * ppm * 1e-6;
            let residual = true_delay - integer as f64 - applied;
            let noise = (hop as f64 * 0.31).sin() * 0.02;
            tracker.observe(applied, 0.2 + residual + noise, 0.2, 0.04);
            let before = integer as f64 + tracker.fraction();
            integer += tracker.fold();
            assert!((before - integer as f64 - tracker.fraction()).abs() < 1e-10);
            if hop * 256 >= 48000 * 5 {
                assert!((integer as f64 + tracker.fraction() - true_delay).abs() < 0.3);
            }
        }
        assert!((tracker.ppm() - ppm).abs() < 1.0);
    }
}

#[test]
fn controller_handles_different_absolute_origins_with_and_without_capture_timestamps() {
    use crate::{ring::SampleRing, StreamFormat};
    for rate in [44100, 48000] {
        let pcm: Vec<_> = noise(rate as usize * 5, 43)
            .iter()
            .map(|value| (value * 16000.0) as i16)
            .collect();
        for timestamps in [false, true] {
            let config = EngineConfig::specified();
            let format = StreamFormat {
                sample_rate: rate,
                ref_channels: 1,
                mic_channels: 1,
            };
            let mut aligner = Aligner::new(config, format, false).unwrap();
            let mut reference = SampleRing::new(rate, 1, 4).unwrap();
            let mut microphone = SampleRing::new(rate, 1, 4).unwrap();
            let block = rate as usize / 100;
            let mut locked_at = None;
            for first in (block..pcm.len() - block).step_by(block) {
                let time = timestamps.then_some(first as i64 * 1_000_000_000 / rate as i64);
                reference
                    .push(
                        10_000_000 + first as u64,
                        &pcm[first..first + block],
                        time,
                        false,
                    )
                    .unwrap();
                microphone
                    .push(
                        7_000_000 + first as u64,
                        &pcm[first - 112..first + block - 112],
                        time,
                        false,
                    )
                    .unwrap();
                if aligner.try_align(&reference, &microphone).is_some() {
                    locked_at = Some(first);
                    break;
                }
            }
            assert!(locked_at
                .is_some_and(|frame| frame < rate as usize * if timestamps { 3 } else { 5 }));
            assert!((aligner.lag() + 3_000_000 - 112).abs() <= 1);
            assert!(aligner.begin_hop(8_000_000).is_some());
            assert!(aligner.begin_hop(8_000_000).is_none());
            aligner.reset();
            assert!(!aligner.locked());
        }
    }
}

#[test]
fn sparse_alignment_locks_through_short_gaps_but_rejects_long_gaps_and_unrelated_signals() {
    use crate::{ring::SampleRing, StreamFormat};
    for rate in [44100, 48000] {
        let signal = noise(rate as usize * 6, 71);
        let unrelated = noise(signal.len(), 97);
        let pcm = |values: &[f32]| {
            values
                .iter()
                .map(|v| (v * 16000.0) as i16)
                .collect::<Vec<_>>()
        };
        let reference_pcm = pcm(&signal);
        let unrelated_pcm = pcm(&unrelated);
        let block = rate as usize / 100;
        for (lag, missing_blocks, related, tolerate) in [
            (-128, 1, true, true),
            (112, 1, true, true),
            (112, 1, true, false),
            (112, 2, true, true),
            (112, 10, true, true),
            (112, 1, false, true),
        ] {
            let config = EngineConfig {
                align_max_missing_fraction: if tolerate { 0.02 } else { 0.0 },
                ..EngineConfig::specified()
            };
            let mut aligner = Aligner::new(
                config,
                StreamFormat {
                    sample_rate: rate,
                    ref_channels: 1,
                    mic_channels: 1,
                },
                false,
            )
            .unwrap();
            let mut reference = SampleRing::new(rate, 1, 4).unwrap();
            let mut microphone = SampleRing::new(rate, 1, 4).unwrap();
            let mut locked_at = None;
            for first in (block..reference_pcm.len() - block).step_by(block) {
                let number = first / block;
                if (60..60 + missing_blocks).contains(&(number % 80)) {
                    continue;
                }
                let mic: Vec<_> = (first..first + block)
                    .map(|i| {
                        if related {
                            reference_pcm[(i as i64 - lag) as usize]
                        } else {
                            unrelated_pcm[i]
                        }
                    })
                    .collect();
                let ns = Some(first as i64 * 1_000_000_000 / rate as i64);
                reference
                    .push(
                        first as u64,
                        &reference_pcm[first..first + block],
                        ns,
                        false,
                    )
                    .unwrap();
                microphone.push(first as u64, &mic, ns, false).unwrap();
                if aligner.try_align(&reference, &microphone).is_some() {
                    locked_at = Some(first);
                    break;
                }
            }
            if related && missing_blocks == 1 && tolerate {
                assert!(locked_at.is_some_and(|frame| frame < rate as usize * 3));
                assert!((aligner.lag() - lag).abs() <= 1);
            } else {
                assert!(locked_at.is_none());
            }
        }
    }
}

#[test]
fn rejected_alignment_coverage_breaks_a_pending_pair_of_good_estimates() {
    use crate::{ring::SampleRing, StreamFormat};
    for rate in [44100, 48000] {
        let pcm: Vec<_> = noise(rate as usize * 6, 101)
            .iter()
            .map(|v| (v * 16000.0) as i16)
            .collect();
        let mut aligner = Aligner::new(
            EngineConfig::specified(),
            StreamFormat {
                sample_rate: rate,
                ref_channels: 1,
                mic_channels: 1,
            },
            false,
        )
        .unwrap();
        let mut reference = SampleRing::new(rate, 1, 4).unwrap();
        let mut microphone = SampleRing::new(rate, 1, 4).unwrap();
        let block = rate as usize / 100;
        let mut locked_at = None;
        for first in (0..pcm.len() - block).step_by(block) {
            if (160..165).contains(&(first / block)) {
                continue;
            }
            let mic: Vec<_> = (first..first + block)
                .map(|i| i.checked_sub(112).map_or(0, |j| pcm[j]))
                .collect();
            let ns = Some(first as i64 * 1_000_000_000 / rate as i64);
            reference
                .push(first as u64, &pcm[first..first + block], ns, false)
                .unwrap();
            microphone.push(first as u64, &mic, ns, false).unwrap();
            if aligner.try_align(&reference, &microphone).is_some() {
                locked_at = Some(first);
                break;
            }
        }
        // The first good estimate precedes the 50 ms gap. The rejected search must erase it;
        // two fresh estimates after that gap lock around 4.33 s rather than 3.33 s.
        assert!(locked_at.is_some_and(|at| (rate as usize * 4..rate as usize * 5).contains(&at)));
        assert!((aligner.lag() - 112).abs() <= 1);
    }
}
