//! Development probe: how well the engine sees a hand moving straight toward the phone and back
//! at fixed speeds (cm/s), with no hover beforehand. It prints derived measurements only.
//! `--set=key=value` overrides a parameter of the short-change candidate (decision 0021).
use std::{env, error::Error};
use veyl_dsp::config::EngineConfig;
use veyl_lab::{analysis, recording::Input};
use veyl_sim::scene::{self, Mover, Scene};

fn main() -> Result<(), Box<dyn Error>> {
    let mut overrides = vec![];
    let mut speeds = vec![];
    for argument in env::args().skip(1) {
        if let Some(value) = argument.strip_prefix("--set=") {
            let (key, value) = value.split_once('=').ok_or("Use --set=key=value.")?;
            overrides.push((key.to_owned(), value.to_owned()));
        } else {
            speeds.push(argument.parse::<f64>()?);
        }
    }
    if speeds.is_empty() {
        speeds = vec![8.0, 30.0, 50.0, 80.0];
    }
    for speed in speeds {
        let mut scene = Scene::phone(1, 14.0);
        let (top, bottom, start) = (0.32, 0.10, 6.0);
        let travel = (top - bottom) / (speed / 100.0);
        scene.movers.push(Mover::Path(vec![
            (0.0, [0.0, 0.0, top]),
            (start, [0.0, 0.0, top]),
            (start + travel, [0.0, 0.0, bottom]),
            (start + travel + 1.0, [0.0, 0.0, bottom]),
            (start + 2.0 * travel + 1.0, [0.0, 0.0, top]),
        ]));
        let rendered = scene::render(&scene)?;
        let truth = rendered.truth.clone();
        let mut input = Input::generated(rendered);
        input.meta.device["mode"] = "phone".into();
        input.meta.device["sound_source"] = "test_sound".into();
        input.meta.device["route"] = "speaker".into();
        input.meta.device["media_volume"] = scene.volume.into();
        let mut config = serde_json::to_value(EngineConfig {
            infer_single_path: true,
            resample_reference: true,
            residual_motion: true,
            floor_relative: 1e-8,
            impulse_ratio: 12.0,
            impulse_relative: true,
            change_window: 1024,
            change_smooth_bins: 8,
            track_gate_cm: 4.0,
            ..analysis::default_config(&input)
        })?;
        for (key, value) in &overrides {
            if config.get(key).is_none() {
                return Err(format!("Unknown engine parameter {key}.").into());
            }
            config[key] = serde_json::from_str(value)
                .unwrap_or_else(|_| serde_json::Value::String(value.clone()));
        }
        let config: EngineConfig = serde_json::from_value(config)?;
        config.validate()?;
        let run = analysis::evaluate(&input, config, true)?;
        let rate = scene.fs as f64;
        let (mut sum, mut hops, mut present, mut on_hand) = (0.0, 0, 0, 0);
        for row in &run.features {
            let t = row.feature.frame as f64 / rate;
            let actual = truth
                .iter()
                .filter(|sample| sample.mover == 0)
                .min_by_key(|sample| sample.frame.abs_diff(row.feature.frame))
                .map_or(f32::NAN, |sample| sample.echo_distance_m * 100.0);
            let moving = (t > start + 0.05 && t < start + travel - 0.05)
                || (t > start + travel + 1.05 && t < start + 2.0 * travel + 0.95);
            if moving && (12.0..32.0).contains(&actual) {
                sum += row.feature.z_max;
                hops += 1;
                present += usize::from(row.feature.present);
                on_hand += usize::from(
                    row.feature
                        .r_peak_cm
                        .is_some_and(|r| (r - actual).abs() < 2.0),
                );
            }
        }
        let share = |count: usize| 100.0 * count as f64 / hops.max(1) as f64;
        println!(
            "{speed:>5.0} cm/s: {hops} moving hops, mean z {:.1}, present {:.0}%, peak on the hand {:.0}%",
            sum / hops.max(1) as f32,
            share(present),
            share(on_hand),
        );
    }
    Ok(())
}
