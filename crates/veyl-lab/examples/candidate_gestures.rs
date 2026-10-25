//! Development probe for gesture gates. This is not the completed M4 acceptance suite.
//! `--set=key=value` overrides an engine parameter; `--explain` describes each missed poke
//! from the derived features only; `--trace=START..END` prints each hop's features beside the
//! simulator's true echo distance.
use std::{env, error::Error};
use veyl_dsp::{config::EngineConfig, Sensitivity};
use veyl_lab::{analysis, recording::Input};
use veyl_sim::{scenarios, scene};

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = env::args().skip(1).collect();
    let seed: u64 = arguments
        .first()
        .map(String::as_str)
        .unwrap_or("1")
        .parse()?;
    let mut sensitivity = Sensitivity::Normal;
    let mut selected = false;
    let mut cases = vec![];
    let mut overrides = vec![];
    let mut explain = false;
    let mut trace = None;
    for argument in arguments.iter().skip(1) {
        if argument == "--explain" {
            explain = true;
        } else if let Some(value) = argument.strip_prefix("--trace=") {
            let (start, end) = value.split_once("..").ok_or("Use --trace=START..END.")?;
            trace = Some((start.parse::<f64>()?, end.parse::<f64>()?));
        } else if let Some(value) = argument.strip_prefix("--set=") {
            let (key, value) = value.split_once('=').ok_or("Use --set=key=value.")?;
            overrides.push((key.to_owned(), value.to_owned()));
        } else if let Some(value) = argument.strip_prefix("--sensitivity=") {
            if selected {
                return Err("Choose sensitivity once for the whole run.".into());
            }
            sensitivity = match value {
                "low" => Sensitivity::Low,
                "normal" => Sensitivity::Normal,
                "high" => Sensitivity::High,
                _ => return Err("Sensitivity must be low, normal or high.".into()),
            };
            selected = true;
        } else {
            cases.push(argument.as_str());
        }
    }
    if cases.is_empty() {
        cases = vec![
            "still",
            "poke_basic",
            "poke_varied",
            "hover_drift",
            "wave",
            "walk_by",
            "typing",
            "typing_pokes",
            "volume_steps",
            "drift_50ppm",
            "drift_200ppm",
            "dropouts",
            "compressed16k",
            "gappy",
            "blocked",
            "stereo_speakers",
            "pc_mode",
            "pc_bluetooth",
        ];
    }
    let (poke_in_cm, poke_out_cm, poke_v_min) = sensitivity.poke_thresholds();
    let mut failed = 0;
    for name in cases {
        let scene = scenarios::scene(name, seed, None)?;
        let rendered = scene::render(&scene)?;
        // A diagnostic, not a gate: the truth pokes whose echo-distance depth and speeds meet
        // the selected sensitivity's own thresholds. Truth geometry, never sensing output,
        // decides.
        let detectable: Vec<u64> = match &scene.movers.first() {
            Some(scene::Mover::Pokes(gestures)) => gestures
                .iter()
                .zip(&rendered.labels)
                .filter(|(gesture, label)| {
                    label.depth_cm >= poke_in_cm
                        && label.depth_cm as f64 * 1000.0 / gesture.approach_ms >= poke_v_min as f64
                        && label.depth_cm as f64 * 1000.0 / gesture.retreat_ms >= poke_v_min as f64
                })
                .map(|(_, label)| label.turn_frame)
                .collect(),
            _ => vec![],
        };
        let truth = rendered.truth.clone();
        let mut input = Input::generated(rendered);
        let pc = name.starts_with("pc_");
        input.meta.device["mode"] = if pc { "pc" } else { "phone" }.into();
        input.meta.device["sound_source"] = if name == "blocked" {
            "capture"
        } else if pc {
            "pc"
        } else {
            "test_sound"
        }
        .into();
        input.meta.device["route"] = if name == "pc_bluetooth" {
            "bluetooth"
        } else {
            "speaker"
        }
        .into();
        input.meta.device["media_volume"] = scene.volume.into();
        // The short-change candidate (decision 0021). The residual candidate of decision 0020 is
        // --set=change_window=0 --set=change_smooth_bins=0 --set=track_gate_cm=0
        // --set=impulse_relative=false. The earlier 5 ms candidate additionally sets
        // --set=residual_motion=false --set=fast_seconds=0.005 --set=floor_relative=1e-6
        // --set=impulse_ratio=6.
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
            z_on: sensitivity.z_on(),
            poke_in_cm,
            poke_out_cm,
            poke_v_min,
            ..analysis::default_config(&input)
        })?;
        for (key, value) in &overrides {
            let parsed: serde_json::Value = serde_json::from_str(value)
                .unwrap_or_else(|_| serde_json::Value::String(value.clone()));
            if config.get(key).is_none() {
                return Err(format!("Unknown engine parameter {key}.").into());
            }
            config[key] = parsed;
        }
        let config: EngineConfig = serde_json::from_value(config)?;
        config.validate()?;
        let run = analysis::evaluate(&input, config, true)?;
        let pokes = &run.report.pokes;
        if let Some((start, end)) = trace {
            // Derived features beside the simulator's true echo distance, one line per hop.
            let rate = scene.fs as f64;
            for row in &run.features {
                let t = row.feature.frame as f64 / rate;
                if t < start || t > end {
                    continue;
                }
                let actual = truth
                    .iter()
                    .filter(|sample| sample.mover == 0)
                    .min_by_key(|sample| sample.frame.abs_diff(row.feature.frame))
                    .map(|sample| sample.echo_distance_m * 100.0);
                let f = &row.feature;
                println!(
                    "{t:.3} truth {:>5.1} peak {:>5} track {:>5} z {:>6.1} peaky {:>5.1} present {} {}",
                    actual.unwrap_or(f32::NAN),
                    f.r_peak_cm.map_or("-".into(), |v| format!("{v:.1}")),
                    f.distance_cm.map_or("-".into(), |v| format!("{v:.1}")),
                    f.z_max,
                    f.peakiness,
                    u8::from(f.present),
                    if !f.valid {
                        "held"
                    } else if row.suspended {
                        "suspended"
                    } else {
                        ""
                    },
                );
            }
            for poke in &run.pokes {
                let t = poke.turn_frame as f64 / rate;
                if t >= start && t <= end {
                    println!(
                        "poke turn {t:.3} depth {:.1} at {:.1}",
                        poke.depth_cm, poke.distance_cm
                    );
                }
            }
        }
        if explain {
            let rate = scene.fs as f64;
            for label in input
                .labels
                .iter()
                .filter(|label| label.kind == "poke_truth")
            {
                let turn = label.frame as f64 / rate;
                if run
                    .pokes
                    .iter()
                    .any(|poke| (poke.turn_frame as f64 / rate - turn).abs() <= 0.15)
                {
                    continue;
                }
                let rows: Vec<_> = run
                    .features
                    .iter()
                    .filter(|row| {
                        let t = row.feature.frame as f64 / rate;
                        t >= turn - 1.0 && t <= turn + 0.5
                    })
                    .collect();
                let near = rows.iter().filter(|row| {
                    let t = row.feature.frame as f64 / rate;
                    (t - turn).abs() <= 0.15 && row.feature.distance_cm.is_some()
                });
                let Some(minimum) = near.min_by(|a, b| {
                    a.feature
                        .distance_cm
                        .unwrap()
                        .total_cmp(&b.feature.distance_cm.unwrap())
                }) else {
                    let present = rows.iter().filter(|row| row.feature.present).count();
                    let peak = rows
                        .iter()
                        .map(|row| row.feature.z_max)
                        .fold(0.0_f32, f32::max);
                    let suspended = rows.iter().filter(|row| row.suspended).count();
                    println!("  miss {turn:.2}s: no hand near turn; present {present}/{} hops, z_max {peak:.1}, suspended {suspended}", rows.len());
                    continue;
                };
                let tm = minimum.feature.frame as f64 / rate;
                let closest = minimum.feature.distance_cm.unwrap();
                let before: Vec<_> = rows
                    .iter()
                    .filter(|row| {
                        let t = row.feature.frame as f64 / rate;
                        t <= tm && t >= tm - 0.9 && row.feature.distance_cm.is_some()
                    })
                    .collect();
                let approach = before
                    .iter()
                    .max_by(|a, b| {
                        a.feature
                            .distance_cm
                            .unwrap()
                            .total_cmp(&b.feature.distance_cm.unwrap())
                    })
                    .unwrap();
                let ta = approach.feature.frame as f64 / rate;
                let depth = approach.feature.distance_cm.unwrap() - closest;
                let after = rows
                    .iter()
                    .filter(|row| {
                        let t = row.feature.frame as f64 / rate;
                        t > tm && t <= tm + 0.5 && row.feature.distance_cm.is_some()
                    })
                    .map(|row| row.feature.distance_cm.unwrap() - closest)
                    .fold(f32::MIN, f32::max);
                let suspended = rows
                    .iter()
                    .filter(|row| {
                        row.suspended && (row.feature.frame as f64 / rate - tm).abs() < 0.3
                    })
                    .count();
                println!(
                    "  miss {turn:.2}s: turn {tm:.3} at {closest:.1} cm z {:.1}; approach from {:.1} cm at {ta:.3} (depth {depth:.1}, {:.0} cm/s); max rise {after:.1}; suspended near turn {suspended}",
                    minimum.feature.z_max,
                    approach.feature.distance_cm.unwrap(),
                    depth / (tm - ta).max(1e-3) as f32,
                );
            }
        }
        // One-to-one matches within the simulator's 0.15 s truth window.
        let window = (0.15 * scene.fs as f64) as u64;
        let mut used = vec![false; run.pokes.len()];
        let detectable_matched = detectable
            .iter()
            .filter(|&&truth| {
                let found = (0..run.pokes.len())
                    .filter(|&i| !used[i] && run.pokes[i].turn_frame.abs_diff(truth) <= window)
                    .min_by_key(|&i| run.pokes[i].turn_frame.abs_diff(truth));
                if let Some(i) = found {
                    used[i] = true;
                }
                found.is_some()
            })
            .count();
        let mut failures = vec![];
        let mut require = |passes: bool, condition: &'static str| {
            if !passes {
                failures.push(condition);
            }
        };
        match name {
            "still" | "walk_by" | "blocked" => require(pokes.detected == 0, "zero pokes"),
            "hover_drift" | "wave" | "typing" => {
                require(pokes.detected <= 1, "at most one false poke")
            }
            "poke_basic" => {
                require(pokes.recall.is_some_and(|v| v >= 0.95), "recall >= 95%");
                require(
                    pokes.precision.is_some_and(|v| v >= 0.95),
                    "precision >= 95%",
                );
                require(
                    pokes.latency_median_ms.is_some_and(|v| v <= 80.0),
                    "median truth-to-detect <= 80 ms",
                );
            }
            "poke_varied" => {
                require(pokes.recall.is_some_and(|v| v >= 0.9), "recall >= 90%");
                require(
                    pokes.precision.is_some_and(|v| v >= 0.95),
                    "precision >= 95%",
                );
            }
            "typing_pokes" => require(pokes.recall.is_some_and(|v| v >= 0.85), "recall >= 85%"),
            "volume_steps" | "pc_mode" => {
                require(pokes.recall.is_some_and(|v| v >= 0.9), "recall >= 90%");
                require(
                    pokes.false_pokes.is_some_and(|v| v <= 1),
                    "at most one false poke",
                );
            }
            "drift_50ppm" | "drift_200ppm" => {
                require(pokes.recall.is_some_and(|v| v >= 0.95), "recall >= 95%");
                require(pokes.false_pokes == Some(0), "zero false pokes");
                require(
                    run.report
                        .engine
                        .max_direct_residual_after_5s
                        .is_some_and(|v| v <= 0.3),
                    "direct residual <= 0.3 taps after 5 s",
                );
                require(run.report.engine.drift_warnings == 0, "no drift warning");
            }
            "dropouts" => {
                require(pokes.recall.is_some_and(|v| v >= 0.85), "recall >= 85%");
                require(pokes.false_pokes == Some(0), "zero false pokes");
            }
            "compressed16k" => require(pokes.recall.is_some_and(|v| v >= 0.9), "recall >= 90%"),
            "stereo_speakers" => require(pokes.recall.is_some_and(|v| v >= 0.95), "recall >= 95%"),
            "pc_bluetooth" => {
                require(pokes.recall.is_some_and(|v| v >= 0.9), "recall >= 90%");
                require(
                    run.report.engine.lock_ms.is_some_and(|v| v <= 3000.0),
                    "lock <= 3 s",
                );
            }
            "gappy" => {
                require(
                    !run.pokes.iter().any(|poke| {
                        let seconds = poke.detect_frame as f64 / scene.fs as f64;
                        (seconds % 4.0) >= 3.0
                    }),
                    "no detections during source gaps",
                );
            }
            _ => return Err("Choose a scenario with gesture gates.".into()),
        }
        let passed = failures.is_empty();
        failed += usize::from(!passed);
        println!(
            "{}",
            serde_json::json!({"scenario":name,"seed":seed,"seconds":scene.seconds,"sensitivity":sensitivity,
            "passed_gesture_checks":passed,"failures":failures,"pokes":pokes,"engine":run.report.engine,
            "detectable":{"truth":detectable.len(),"matched":detectable_matched}})
        );
    }
    if failed > 0 {
        return Err(format!(
            "{failed} scenario gesture checks failed; M4 acceptance remains open."
        )
        .into());
    }
    Ok(())
}
