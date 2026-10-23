//! Development probe for gesture gates. This is not the completed M4 acceptance suite.
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
    for argument in arguments.iter().skip(1) {
        if let Some(value) = argument.strip_prefix("--sensitivity=") {
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
        let mut input = Input::generated(scene::render(&scene)?);
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
        let run = analysis::evaluate(
            &input,
            EngineConfig {
                fast_seconds: 0.005,
                infer_single_path: true,
                resample_reference: true,
                z_on: sensitivity.z_on(),
                poke_in_cm,
                poke_out_cm,
                poke_v_min,
                ..analysis::default_config(&input)
            },
            true,
        )?;
        let pokes = &run.report.pokes;
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
            "passed_gesture_checks":passed,"failures":failures,"pokes":pokes,"engine":run.report.engine})
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
