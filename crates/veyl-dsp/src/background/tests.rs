use super::*;
fn response(config: &EngineConfig, echo: f32, gain: Complex<f32>) -> Vec<Complex<f32>> {
    (0..=config.window / 2)
        .map(|bin| {
            gain * (Complex::new(0.7, 0.0)
                + Complex::from_polar(
                    echo,
                    -std::f32::consts::TAU * bin as f32 * 60.0 / config.window as f32,
                ))
        })
        .collect()
}
fn weights(config: &EngineConfig) -> Vec<f32> {
    (0..=config.window / 2)
        .map(|bin| {
            if (8000.0..20000.0).contains(&(bin as f32 * 48000.0 / config.window as f32)) {
                1.0
            } else {
                0.0
            }
        })
        .collect()
}
#[test]
fn direct_normalization_removes_volume_and_phase_but_preserves_and_rejects_model_jumps() {
    let config = EngineConfig::specified();
    let format = StreamFormat {
        sample_rate: 48000,
        ref_channels: 1,
        mic_channels: 1,
    };
    let mut background = Background::new(config.clone(), format).unwrap();
    let weights = weights(&config);
    let mut transform = MapTransform::new(&config).unwrap();
    let initial = response(&config, 0.1, Complex::new(1.0, 0.0));
    let mut maps = vec![Complex::default(); config.pre_taps + config.taps];
    transform.render(&initial, &weights, &mut maps).unwrap();
    assert!(
        background
            .process(&initial, &maps, &weights, false, false)
            .unwrap()
            .settling
    );
    while !background.settled() {
        background
            .process(&initial, &maps, &weights, false, false)
            .unwrap();
    }
    let original = background.transfer().to_vec();
    let volume = response(&config, 0.1, Complex::from_polar(2.0, 0.4));
    transform.render(&volume, &weights, &mut maps).unwrap();
    let adjusted = background
        .process(&volume, &maps, &weights, false, false)
        .unwrap();
    assert!(!adjusted.model_jump && !adjusted.settling);
    assert!(background.dynamic().iter().all(|tap| tap.norm() < 1e-6));
    assert!(background
        .transfer()
        .iter()
        .zip(&original)
        .all(|(a, b)| (*a - *b).norm() < 1e-6));
    for gain in [Complex::new(5.0, 0.0), Complex::from_polar(2.0, 3.0)] {
        let changed = response(&config, 0.1, gain);
        transform.render(&changed, &weights, &mut maps).unwrap();
        assert!(
            background
                .process(&changed, &maps, &weights, false, false)
                .unwrap()
                .model_jump
        );
        assert!(background
            .transfer()
            .iter()
            .zip(&original)
            .all(|(a, b)| (*a - *b).norm() < 1e-6));
    }
    background.reset(true);
    assert!(!background.settled());
    assert!(background
        .transfer()
        .iter()
        .all(|tap| tap.norm_sqr() == 0.0));
}

#[test]
fn presence_slows_learning_only_temporarily_and_impulses_hold_the_model() {
    let config = EngineConfig {
        settle_seconds: 0.02,
        presence_hold_seconds: 0.1,
        ..EngineConfig::specified()
    };
    let format = StreamFormat {
        sample_rate: 48000,
        ref_channels: 1,
        mic_channels: 1,
    };
    let mut background = Background::new(config.clone(), format).unwrap();
    let weights = weights(&config);
    let mut transform = MapTransform::new(&config).unwrap();
    let initial = response(&config, 0.0, Complex::new(1.0, 0.0));
    let changed = response(&config, 0.2, Complex::new(1.0, 0.0));
    let mut maps = vec![Complex::default(); config.pre_taps + config.taps];
    transform.render(&initial, &weights, &mut maps).unwrap();
    while !background.settled() {
        background
            .process(&initial, &maps, &weights, false, false)
            .unwrap();
    }
    transform.render(&changed, &weights, &mut maps).unwrap();
    let original = background.transfer().to_vec();
    background
        .process(&changed, &maps, &weights, true, true)
        .unwrap();
    assert!(background.transfer() == original);
    background
        .process(&changed, &maps, &weights, true, false)
        .unwrap();
    let early = background
        .transfer()
        .iter()
        .zip(&original)
        .map(|(a, b)| (*a - *b).norm_sqr())
        .sum::<f32>()
        .sqrt();
    for _ in 0..20 {
        background
            .process(&changed, &maps, &weights, true, false)
            .unwrap();
    }
    let before = background.transfer().to_vec();
    background
        .process(&changed, &maps, &weights, true, false)
        .unwrap();
    let late = background
        .transfer()
        .iter()
        .zip(&before)
        .map(|(a, b)| (*a - *b).norm_sqr())
        .sum::<f32>()
        .sqrt();
    assert!(late > early * 5.0);
    assert!(background.dynamic()[config.pre_taps + 60].norm() > 0.05);
}

#[test]
fn distance_origin_keeps_a_strong_direct_path_when_microphones_exchange_peak_rank() {
    let config = EngineConfig::specified();
    let format = StreamFormat {
        sample_rate: 48000,
        ref_channels: 1,
        mic_channels: 1,
    };
    let mut background = Background::new(config.clone(), format).unwrap();
    let weights = weights(&config);
    let mut transform = MapTransform::new(&config).unwrap();
    let mut maps = vec![Complex::default(); config.pre_taps + config.taps];
    let spectrum = |first: f32, second: f32| {
        (0..=config.window / 2)
            .map(|bin| {
                Complex::new(first, 0.0)
                    + Complex::from_polar(
                        second,
                        std::f32::consts::TAU * bin as f32 * 15.0 / config.window as f32,
                    )
            })
            .collect::<Vec<_>>()
    };
    for (first, second, expected) in [(1.0, 0.9, 0.0), (0.9, 1.0, 0.0), (0.1, 1.0, -15.0)] {
        let response = spectrum(first, second);
        transform.render(&response, &weights, &mut maps).unwrap();
        background
            .process(&response, &maps, &weights, false, false)
            .unwrap();
        let result = background
            .process(&response, &maps, &weights, false, false)
            .unwrap();
        assert!((result.direct.position - expected).abs() < 0.3);
    }
    background.reset(true);
    let response = spectrum(1.0, 0.9);
    transform.render(&response, &weights, &mut maps).unwrap();
    let result = background
        .process(&response, &maps, &weights, false, false)
        .unwrap();
    assert!(result.direct.position.abs() < 0.3);
}

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

#[test]
fn residual_change_learns_correlated_stereo_paths_and_finds_a_new_echo() {
    let config = EngineConfig {
        residual_motion: true,
        settle_seconds: 0.3,
        ..EngineConfig::specified()
    };
    let format = StreamFormat {
        sample_rate: 48000,
        ref_channels: 2,
        mic_channels: 1,
    };
    let (n, hop, pre) = (config.window, config.hop, config.pre_taps);
    let length = 48000 * 6;
    // Strongly correlated channels make a per-hop 2x2 inverse noisy.
    let shared = noise(length, 0x1234_5678_9abc_def1);
    let own = noise(length, 0x0fed_cba9_8765_4321);
    let left = shared.clone();
    let right: Vec<_> = shared
        .iter()
        .zip(&own)
        .map(|(s, o)| 0.8 * s + 0.6 * o)
        .collect();
    let echo_start = 48000 * 5;
    let microphone: Vec<_> = (0..length)
        .map(|i| {
            let at = |signal: &[f32], delay: usize| i.checked_sub(delay).map_or(0.0, |j| signal[j]);
            let mut value = at(&left, 0) + 0.7 * at(&right, 3) + 0.3 * at(&left, 10);
            if i >= echo_start {
                // A new reflector returns both channels 60 taps after the direct sound.
                value += 0.02 * (at(&left, 60) + at(&right, 60));
            }
            value
        })
        .collect();
    let mut echo = EchoEstimator::new(config.clone(), format).unwrap();
    let mut background = Background::new(config.clone(), format).unwrap();
    let mut reference = vec![0.0; 2 * n];
    let mut quiet = 0.0_f32;
    let mut found = None;
    let mut end = n;
    while end <= length {
        reference[..n].copy_from_slice(&left[end - n..end]);
        reference[n..].copy_from_slice(&right[end - n..end]);
        let fast = echo
            .process(&reference, &microphone[end - n..end], 0.0, None, false)
            .unwrap();
        let frame = background
            .process_residual(&echo, echo.weights(), false, false)
            .unwrap();
        let taps = background.dynamic();
        if !frame.settling && end >= 48000 * 4 && end < echo_start {
            assert_eq!(fast.mode, crate::echo::ReferenceMode::Stereo);
            quiet = taps[pre + 20..]
                .iter()
                .map(|tap| tap.norm())
                .fold(quiet, f32::max);
        }
        if end >= echo_start + n && found.is_none() {
            let (index, tap) = taps
                .iter()
                .enumerate()
                .take(pre + config.taps)
                .skip(pre + 20)
                .max_by(|a, b| a.1.norm().total_cmp(&b.1.norm()))
                .unwrap();
            // Relative to the mean reference, the returned sum of both channels has gain 0.04.
            let expected: Vec<_> = (0..=n / 2)
                .map(|bin| {
                    Complex::from_polar(0.04, -std::f32::consts::TAU * bin as f32 * 60.0 / n as f32)
                })
                .collect();
            let mut map = vec![Complex::default(); pre + config.taps];
            MapTransform::new(&config)
                .unwrap()
                .render(&expected, echo.weights(), &mut map)
                .unwrap();
            found = Some((index - pre, tap.norm(), map[pre + 60].norm()));
        }
        end += hop;
    }
    let (delay, size, expected) = found.unwrap();
    assert!((59..=61).contains(&delay), "new echo at tap {delay}");
    assert!(
        (size / expected - 1.0).abs() < 0.2,
        "new echo {size} against {expected}"
    );
    assert!(
        quiet < size * 0.5,
        "unchanged scene {quiet} against echo {size}"
    );
}

#[test]
fn short_change_keeps_a_fast_moving_echo_that_the_whole_window_cancels() {
    let format = StreamFormat {
        sample_rate: 48000,
        ref_channels: 1,
        mic_channels: 1,
    };
    let length = 48000 * 3;
    let reference = noise(length, 0x5eed_0fec_4012_3456);
    let start = 48000 * 2;
    // One tap closer every 256 samples: about 67 cm/s of echo distance, a quick poke.
    let delay = |i: usize| 120 - ((i.saturating_sub(start)) / 256).min(80);
    let microphone: Vec<_> = (0..length)
        .map(|i| {
            let at = |delay: usize| i.checked_sub(delay).map_or(0.0, |j| reference[j]);
            let mut value = at(0) + 0.3 * at(10);
            if i >= start {
                value += 0.02 * at(delay(i));
            }
            value
        })
        .collect();
    let mut peaks = vec![];
    for change_window in [0, 1024] {
        let config = EngineConfig {
            residual_motion: true,
            settle_seconds: 0.3,
            change_window,
            change_smooth_bins: 8,
            ..EngineConfig::specified()
        };
        let (n, hop, pre) = (config.window, config.hop, config.pre_taps);
        let mut echo = EchoEstimator::new(config.clone(), format).unwrap();
        let mut background = Background::new(config.clone(), format).unwrap();
        let (mut sum, mut count, mut located) = (0.0, 0, 0);
        let mut end = n;
        while end <= length {
            echo.process(
                &reference[end - n..end],
                &microphone[end - n..end],
                0.0,
                None,
                false,
            )
            .unwrap();
            background
                .process_residual(&echo, echo.weights(), false, false)
                .unwrap();
            // The moving echo, while every sample of the window sees it move.
            if end >= start + n && delay(end) > 50 {
                let expected = delay(end - n / 2) + pre;
                let taps = background.dynamic();
                sum += (expected - 2..=expected + 2)
                    .map(|k| taps[k].norm())
                    .fold(0.0_f32, f32::max);
                let strongest = (pre + 20..pre + config.taps)
                    .max_by(|a, b| taps[*a].norm().total_cmp(&taps[*b].norm()))
                    .unwrap();
                located += usize::from(strongest.abs_diff(expected) <= 2);
                count += 1;
            }
            end += hop;
        }
        peaks.push((sum / count as f32, located as f32 / count as f32));
    }
    let (whole, short) = (peaks[0], peaks[1]);
    // The whole window spans eight taps of movement and mostly loses the echo; the segment
    // spans four, keeps more of it and always finds where it is.
    assert!(
        short.0 > 1.5 * whole.0 && short.1 > 0.9 && whole.1 < 0.5,
        "moving echo (size, located): short {short:?} against whole window {whole:?}"
    );
}

#[test]
fn a_click_in_the_segment_is_blanked_instead_of_spreading_over_every_tap() {
    let format = StreamFormat {
        sample_rate: 48000,
        ref_channels: 1,
        mic_channels: 1,
    };
    let length = 48000 * 2;
    let reference = noise(length, 0x0c11_c4ed_5eed_0001);
    let start = 48000 + 24000;
    let microphone: Vec<_> = (0..length)
        .map(|i| {
            let at = |delay: usize| i.checked_sub(delay).map_or(0.0, |j| reference[j]);
            let mut value = at(0) + 0.3 * at(10);
            if i >= start {
                // A new reflector, then a click far louder than its echo.
                value += 0.02 * at(60);
            }
            value
        })
        .collect();
    let mut clicked = microphone.clone();
    let click = noise(96, 0xbeef_0000_c11c_0002);
    for (i, value) in click.iter().enumerate() {
        clicked[start + 6000 + i] += value * 3.0 * (-(i as f32) / 24.0).exp();
    }
    let mut contrast = vec![];
    for (input, blank) in [(&microphone, 0.0), (&clicked, 0.0), (&clicked, 4.0)] {
        let config = EngineConfig {
            residual_motion: true,
            settle_seconds: 0.3,
            change_window: 1024,
            change_smooth_bins: 8,
            change_own_fit: true,
            change_in_band: true,
            change_blank_ratio: blank,
            ..EngineConfig::specified()
        };
        let (n, pre) = (config.window, config.pre_taps);
        let mut echo = EchoEstimator::new(config.clone(), format).unwrap();
        let mut background = Background::new(config.clone(), format).unwrap();
        let mut worst = f32::INFINITY;
        let mut end = n;
        while end <= length {
            echo.process(
                &reference[end - n..end],
                &input[end - n..end],
                0.0,
                None,
                false,
            )
            .unwrap();
            background
                .process_residual(&echo, echo.weights(), false, false)
                .unwrap();
            // Hops whose segment contains the click.
            let centre = end - n / 2;
            if centre.abs_diff(start + 6000) < 400 {
                let taps = background.dynamic();
                let echo_size = (pre + 58..=pre + 62)
                    .map(|k| taps[k].norm())
                    .fold(0.0_f32, f32::max);
                let elsewhere = (pre + 20..pre + config.taps)
                    .filter(|k| k.abs_diff(pre + 60) > 4)
                    .map(|k| taps[k].norm())
                    .fold(0.0_f32, f32::max);
                worst = worst.min(echo_size / elsewhere);
            }
            end += config.hop;
        }
        contrast.push(worst);
    }
    let (clean, unblanked, blanked) = (contrast[0], contrast[1], contrast[2]);
    // The click buries the echo; blanking restores most of its contrast.
    assert!(unblanked < 1.0, "unblanked contrast {unblanked}");
    assert!(
        blanked > 0.5 * clean && blanked > 3.0,
        "blanked {blanked} against clean {clean}"
    );
}
