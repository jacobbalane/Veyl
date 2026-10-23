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
