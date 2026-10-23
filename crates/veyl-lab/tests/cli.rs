use std::{fs, process::Command};
#[test]
fn sim_cli_exports_a_loadable_input_and_analyze_reports_it_without_overwriting_files() {
    let root = tempfile::tempdir().unwrap();
    let scene = root.path().join("scene");
    let summary = root.path().join("summary");
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_veyl-lab"))
            .args([
                "sim",
                "pc_bluetooth",
                "--seed",
                "2",
                "--seconds",
                "0.1",
                "--out",
            ])
            .arg(&scene)
            .output()
            .unwrap()
    };
    assert!(run().status.success());
    let before = fs::read(scene.join("meta.json")).unwrap();
    assert!(!run().status.success());
    assert!(before == fs::read(scene.join("meta.json")).unwrap());
    assert!(Command::new(env!("CARGO_BIN_EXE_veyl-lab"))
        .arg("analyze")
        .arg(&scene)
        .arg("--out")
        .arg(&summary)
        .status()
        .unwrap()
        .success());
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(summary.join("summary.json")).unwrap()).unwrap();
    assert_eq!(report["source"], "sim");
    assert_eq!(report["reference"]["blocks"], 10);
    assert_eq!(report["microphone"]["blocks"], 10);
    assert!(!Command::new(env!("CARGO_BIN_EXE_veyl-lab"))
        .args(["sim", "still", "--seconds", "NaN", "--out"])
        .arg(root.path().join("invalid"))
        .output()
        .unwrap()
        .status
        .success());
    assert!(!root.path().join("invalid").exists());
}

#[test]
fn analyze_config_and_sweep_cli_produce_engine_reports_and_preserve_existing_outputs() {
    let root = tempfile::tempdir().unwrap();
    let scene = root.path().join("scene");
    assert!(Command::new(env!("CARGO_BIN_EXE_veyl-lab"))
        .args(["sim", "poke_basic", "--seconds", "10", "--out"])
        .arg(&scene)
        .output()
        .unwrap()
        .status
        .success());
    let config = root.path().join("params.toml");
    fs::write(&config, "fast_seconds = 0.005\ninfer_single_path = true\n").unwrap();
    let destination = root.path().join("analysis");
    let analyze = || {
        Command::new(env!("CARGO_BIN_EXE_veyl-lab"))
            .arg("analyze")
            .arg(&scene)
            .arg("--out")
            .arg(&destination)
            .arg("--config")
            .arg(&config)
            .output()
            .unwrap()
    };
    assert!(analyze().status.success());
    let before = fs::read(destination.join("summary.json")).unwrap();
    let report: serde_json::Value = serde_json::from_slice(&before).unwrap();
    assert_eq!(report["analysis"], "echo_engine");
    assert_eq!(report["pokes"]["matched"], 2);
    assert_eq!(report["pokes"]["recall"], 1.0);
    assert_eq!(report["config"]["infer_single_path"], true);
    assert!(!analyze().status.success());
    assert!(before == fs::read(destination.join("summary.json")).unwrap());
    for file in ["features.csv", "meter.csv", "pokes.csv", "motion.png"] {
        assert!(fs::metadata(destination.join(file)).unwrap().len() > 0);
    }
    let sweep = root.path().join("sweep.csv");
    assert!(Command::new(env!("CARGO_BIN_EXE_veyl-lab"))
        .arg("sweep")
        .arg(&scene)
        .args([
            "--set",
            "fast_seconds=0.005,0.04",
            "--set",
            "infer_single_path=true",
            "--out"
        ])
        .arg(&sweep)
        .output()
        .unwrap()
        .status
        .success());
    assert_eq!(fs::read_to_string(&sweep).unwrap().lines().count(), 3);
    let invalid = root.path().join("invalid-analysis");
    fs::write(&config, "not_a_parameter = 1\n").unwrap();
    assert!(!Command::new(env!("CARGO_BIN_EXE_veyl-lab"))
        .arg("analyze")
        .arg(&scene)
        .arg("--config")
        .arg(&config)
        .arg("--out")
        .arg(&invalid)
        .output()
        .unwrap()
        .status
        .success());
    assert!(!invalid.exists());
}
