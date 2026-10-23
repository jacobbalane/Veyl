use super::*;

fn noise(length: usize, seed: u64) -> Vec<f32> {
    let mut state = seed;
    (0..length)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            ((state >> 40) as f32 / 16777216.0 - 0.5) * 0.3
        })
        .collect()
}
fn format(rate: u32, refs: u8, mics: u8) -> StreamFormat {
    StreamFormat {
        sample_rate: rate,
        ref_channels: refs,
        mic_channels: mics,
    }
}
fn response(n: usize, paths: &[(usize, f32)]) -> Vec<Complex<f32>> {
    (0..=n / 2)
        .map(|bin| {
            paths
                .iter()
                .map(|&(delay, gain)| {
                    Complex::from_polar(gain, -TAU * bin as f32 * delay as f32 / n as f32)
                })
                .sum()
        })
        .collect()
}
fn relative_map_error(actual: &[Complex<f32>], expected: &[Complex<f32>]) -> f32 {
    (actual
        .iter()
        .zip(expected)
        .map(|(a, b)| (*a - *b).norm_sqr())
        .sum::<f32>()
        / expected.iter().map(|v| v.norm_sqr()).sum::<f32>())
    .sqrt()
}

#[test]
fn analytic_maps_preserve_signed_delays_envelope_phase_and_window_correction() {
    let config = EngineConfig::default();
    let mut transform = MapTransform::new(&config).unwrap();
    let n = config.window;
    let mut weights = vec![0.0; n / 2 + 1];
    weights[350..850].fill(1.0);
    let window: Vec<_> = (0..n)
        .map(|i| 0.5 - 0.5 * (TAU * i as f32 / n as f32).cos())
        .collect();
    let energy = window.iter().map(|w| w * w).sum::<f32>();
    for delay in [-12, 0, 9, 80] {
        let spectrum: Vec<_> = (0..=n / 2)
            .map(|bin| Complex::from_polar(0.7, -TAU * bin as f32 * delay as f32 / n as f32))
            .collect();
        let mut taps = vec![Complex::default(); config.pre_taps + config.taps];
        transform.render(&spectrum, &weights, &mut taps).unwrap();
        for k in [-20_i64, 0, 8, 50, 90, 150] {
            let sum: Complex<f32> = (350..850)
                .map(|bin| {
                    Complex::from_polar(
                        1.4 / n as f32,
                        TAU * bin as f32 * (k - delay) as f32 / n as f32,
                    )
                })
                .sum();
            let shift = k.unsigned_abs() as usize;
            let correction = window[..n - shift]
                .iter()
                .zip(&window[shift..])
                .map(|(a, b)| a * b)
                .sum::<f32>()
                / energy;
            assert!((taps[(k + config.pre_taps as i64) as usize] - sum / correction).norm() < 1e-5);
        }
        let peak = taps
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.norm_sqr().total_cmp(&b.1.norm_sqr()))
            .unwrap()
            .0;
        assert_eq!(peak as i64 - config.pre_taps as i64, delay);
    }
}

#[test]
fn mono_and_independent_stereo_recover_known_echo_paths_at_both_rates() {
    let config = EngineConfig::default();
    for rate in [44100, 48000] {
        let n = config.window;
        let frames = rate as usize / 2;
        let left = noise(frames + n, 7);
        let independent = noise(frames + n, 19);
        for stereo in [false, true] {
            let right = if stereo { &independent } else { &left };
            let mut estimator = EchoEstimator::new(config.clone(), format(rate, 2, 2)).unwrap();
            let mut reference = vec![0.0; n * 2];
            let mut microphone = vec![0.0; n * 2];
            let mut last = None;
            for start in (128..frames).step_by(config.hop) {
                reference[..n].copy_from_slice(&left[start..start + n]);
                reference[n..].copy_from_slice(&right[start..start + n]);
                for i in 0..n {
                    let t = start + i;
                    let a = left[t] * 0.7 + left[t - 47] * 0.25;
                    let b = right[t - 10] * 0.4 + right[t - 80] * 0.15;
                    microphone[i] = a + b;
                    microphone[n + i] = a * 0.8 - b * 0.5;
                }
                last = Some(
                    estimator
                        .process(&reference, &microphone, 0.0, None, false)
                        .unwrap(),
                );
            }
            let last = last.unwrap();
            assert_eq!(
                last.mode,
                if stereo {
                    ReferenceMode::Stereo
                } else {
                    ReferenceMode::Mono
                }
            );
            assert!(last.bandwidth_hz > 11000.0 && last.bandwidth_hz < 12500.0);
            let mut transfer = response(n, &[(0, 0.7), (47, 0.25)]);
            let b = response(n, &[(10, 0.4), (80, 0.15)]);
            if stereo {
                transfer.extend(&b);
                transfer.extend(response(n, &[(0, 0.56), (47, 0.2)]));
                transfer.extend(response(n, &[(10, -0.2), (80, -0.075)]));
            } else {
                for i in 0..estimator.bins() {
                    transfer[i] += b[i];
                }
                transfer.extend(vec![Complex::default(); estimator.bins()]);
                let mut second = response(n, &[(0, 0.56), (47, 0.2)]);
                for i in 0..estimator.bins() {
                    second[i] -= b[i] * 0.5;
                }
                transfer.extend(second);
                transfer.extend(vec![Complex::default(); estimator.bins()]);
            }
            let mut expected = vec![Complex::default(); estimator.maps().len()];
            MapTransform::new(&config)
                .unwrap()
                .render(&transfer, estimator.weights(), &mut expected)
                .unwrap();
            let error = relative_map_error(estimator.maps(), &expected);
            assert!(
                error < 0.12,
                "rate {rate}, stereo {stereo}, relative map error {error}"
            );
            assert!((last.direct.position).abs() < 0.3);
        }
    }
}

#[test]
fn fractional_reference_delay_moves_the_echo_peak_in_the_compensating_direction() {
    let config = EngineConfig::default();
    let signal = noise(config.window + 32, 23);
    let reference = &signal[32..];
    let microphone = &signal[24..24 + config.window];
    let mut peaks = vec![];
    for fraction in [-0.4, 0.0, 0.4] {
        let mut estimator = EchoEstimator::new(config.clone(), format(48000, 1, 1)).unwrap();
        let mut last = None;
        for _ in 0..32 {
            last = Some(
                estimator
                    .process(reference, microphone, fraction, None, false)
                    .unwrap(),
            );
        }
        peaks.push(last.unwrap().direct.position);
    }
    assert!(peaks[0] > peaks[1] && peaks[1] > peaks[2]);
    assert!((peaks[0] - 8.4).abs() < 0.12 && (peaks[2] - 7.6).abs() < 0.12);
}

#[test]
fn activity_masks_reject_silence_and_narrow_sound_and_impulses_hold_three_hops() {
    let config = EngineConfig::default();
    let n = config.window;
    let mut estimator = EchoEstimator::new(config.clone(), format(48000, 1, 1)).unwrap();
    let silence = vec![0.0; n];
    let empty = estimator
        .process(&silence, &silence, 0.0, None, false)
        .unwrap();
    assert_eq!(empty.bandwidth_hz, 0.0);
    assert!(estimator.maps().iter().all(|v| v.norm_sqr() == 0.0));
    let tone: Vec<_> = (0..n)
        .map(|i| (TAU * 512.0 * i as f32 / n as f32).sin() * 0.1)
        .collect();
    let narrow = estimator.process(&tone, &tone, 0.0, None, false).unwrap();
    assert!(narrow.bandwidth_hz < 500.0);
    estimator.reset();
    let reference = noise(n, 29);
    let microphone: Vec<_> = reference.iter().map(|v| v * 0.5).collect();
    for _ in 0..64 {
        estimator
            .process(&reference, &microphone, 0.0, None, false)
            .unwrap();
    }
    let background = estimator.transfer().to_vec();
    for _ in 0..96 {
        estimator
            .process(&reference, &microphone, 0.0, Some(&background), false)
            .unwrap();
    }
    let before = estimator.transfer().to_vec();
    let impulse = noise(n, 31);
    let first = estimator
        .process(&reference, &impulse, 0.0, Some(&background), true)
        .unwrap();
    assert!(first.impulse);
    assert!(estimator.transfer() == before);
    for _ in 0..2 {
        let held = estimator
            .process(&reference, &microphone, 0.0, Some(&background), true)
            .unwrap();
        assert!(held.impulse);
        assert!(estimator.transfer() == before);
    }
    let resumed = estimator
        .process(&reference, &microphone, 0.0, Some(&background), true)
        .unwrap();
    assert!(!resumed.impulse);
    assert!(resumed.predicted_power / resumed.residual_power > 100.0);
}

#[test]
fn coherence_hysteresis_returns_to_mono_and_peak_continuity_does_not_hide_a_missing_path() {
    let config = EngineConfig::default();
    let n = config.window;
    let mut estimator = EchoEstimator::new(config.clone(), format(48000, 2, 1)).unwrap();
    let left = noise(n * 80, 37);
    let right = noise(n * 80, 41);
    let mut input = vec![0.0; n * 2];
    for start in (0..n * 60).step_by(n) {
        input[..n].copy_from_slice(&left[start..start + n]);
        input[n..].copy_from_slice(&right[start..start + n]);
        estimator
            .process(&input, &left[start..start + n], 0.0, None, false)
            .unwrap();
    }
    assert_eq!(estimator.mode, ReferenceMode::Stereo);
    for hop in 0..80 {
        let start = hop * n;
        input[..n].copy_from_slice(&left[start..start + n]);
        input[n..].copy_from_slice(&left[start..start + n]);
        estimator
            .process(&input, &left[start..start + n], 0.0, None, false)
            .unwrap();
    }
    assert_eq!(estimator.mode, ReferenceMode::Mono);
    assert!(estimator.transfer()[estimator.bins()..]
        .iter()
        .all(|v| v.norm_sqr() == 0.0));
    let mut peaks = vec![Complex::default(); config.pre_taps + config.taps];
    peaks[config.pre_taps] = Complex::new(0.8, 0.0);
    peaks[config.pre_taps + 15] = Complex::new(1.0, 0.0);
    let select = |values: &[Complex<f32>]| {
        maps::direct_peak_near(values, config.pre_taps, config.taps, Some(0.0), 2.0, 0.5)
    };
    assert!(select(&peaks).position.abs() < 0.01);
    peaks[config.pre_taps] = Complex::new(0.1, 0.0);
    assert!((select(&peaks).position - 15.0).abs() < 0.01);
}
