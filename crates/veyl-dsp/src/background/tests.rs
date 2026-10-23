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
    let config = EngineConfig::default();
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
        ..EngineConfig::default()
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
    let config = EngineConfig::default();
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
