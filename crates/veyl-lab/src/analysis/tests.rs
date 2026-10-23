use super::*;
use crate::recording::Label;
use std::{fs, io::BufReader};

fn poke(seconds: f64) -> PokeEvent {
    PokeEvent {
        turn_frame: (seconds * 48000.0).round() as u64,
        detect_frame: ((seconds + 0.06) * 48000.0).round() as u64,
        distance_cm: 15.0,
        depth_cm: 5.0,
        confidence: 0.8,
    }
}

#[test]
fn matching_is_one_to_one_with_truth_and_asymmetric_human_windows_without_inventing_human_latency()
{
    let mut input = Input::generated(veyl_sim::poke_basic(1, 0.1).unwrap());
    input.labels = vec![
        Label {
            frame: 48000,
            kind: "poke_mark".into(),
        },
        Label {
            frame: 52800,
            kind: "poke_truth".into(),
        },
    ];
    let measurements = measure_pokes(&input, &[poke(0.5), poke(1.1), poke(5.0)], 480000);
    assert_eq!(measurements.matched, 2);
    assert_eq!(measurements.false_pokes, Some(1));
    assert_eq!(measurements.recall, Some(1.0));
    assert_eq!(measurements.precision, Some(2.0 / 3.0));
    assert_eq!(measurements.false_pokes_per_minute, Some(6.0));
    assert_eq!(measurements.latency_median_ms, Some(60.0));
    assert_eq!(measurements.human_mark_offset_median_ms, Some(500.0));
    input.labels.retain(|label| label.kind == "poke_mark");
    let human = measure_pokes(&input, &[poke(0.4), poke(1.2)], 480000);
    assert_eq!(human.matched, 1);
    assert_eq!(human.latency_median_ms, None);
    assert_eq!(human.confirmation_median_ms, Some(60.0));
    input.labels.clear();
    input.meta.source = "capture".into();
    let unknown = measure_pokes(&input, &[poke(1.0)], 480000);
    assert_eq!(unknown.false_pokes, None);
    assert_eq!(unknown.precision, None);
    assert_eq!(unknown.false_pokes_per_minute, None);
    input.meta.device["annotations_complete"] = true.into();
    let labelled_negative = measure_pokes(&input, &[poke(1.0)], 480000);
    assert_eq!(labelled_negative.false_pokes, Some(1));
    assert_eq!(labelled_negative.false_pokes_per_minute, Some(6.0));
    input.meta.source = "sim".into();
    let negative = measure_pokes(&input, &[poke(1.0)], 480000);
    assert_eq!(negative.false_pokes, Some(1));
    assert_eq!(negative.precision, Some(0.0));
    assert_eq!(negative.recall, None);
}

#[test]
fn percentiles_interpolate_even_medians_and_preserve_empty_measurements() {
    let mut values = [40.0, 10.0, 30.0, 20.0];
    assert_eq!(percentile(&mut values, 0.5), Some(25.0));
    assert!((percentile(&mut values, 0.95).unwrap() - 38.5).abs() < 1e-9);
    assert_eq!(percentile(&mut [], 0.5), None);
}

#[test]
fn analysis_exports_all_artifacts_is_repeatable_and_never_uses_labels_for_sensing_or_overwrites_reports(
) {
    let scene = veyl_sim::scenarios::scene("poke_basic", 1, Some(10.0)).unwrap();
    let mut input = Input::generated(veyl_sim::scene::render(&scene).unwrap());
    let config = EngineConfig {
        fast_seconds: 0.005,
        infer_single_path: true,
        ..default_config(&input)
    };
    let run = evaluate(&input, config.clone(), true).unwrap();
    assert_eq!(run.report.pokes.matched, 2);
    assert_eq!(run.report.pokes.false_pokes, Some(0));
    assert!(run.report.engine.lock_ms.unwrap() <= 3000.0);
    assert!(run.features.len() > 600 && run.meters.len() > 80);
    assert!(run.report.pokes.latency_median_ms.unwrap() < 200.0);
    let repeated = evaluate(&input, config.clone(), true).unwrap();
    assert!(
        serde_json::to_vec(&run.report).unwrap() == serde_json::to_vec(&repeated.report).unwrap()
    );
    let original_labels = std::mem::take(&mut input.labels);
    let unlabelled = evaluate(&input, config, true).unwrap();
    assert!(
        serde_json::to_vec(&run.pokes).unwrap() == serde_json::to_vec(&unlabelled.pokes).unwrap()
    );
    assert!(run
        .features
        .iter()
        .zip(&unlabelled.features)
        .all(
            |(a, b)| a.feature.z_max.to_bits() == b.feature.z_max.to_bits()
                && a.feature.distance_cm.map(f32::to_bits)
                    == b.feature.distance_cm.map(f32::to_bits)
        ));
    input.labels = original_labels;
    let folder = tempfile::tempdir().unwrap();
    write_outputs(&input, &run, folder.path()).unwrap();
    for file in [
        "summary.json",
        "features.csv",
        "meter.csv",
        "pokes.csv",
        "motion.png",
    ] {
        assert!(fs::metadata(folder.path().join(file)).unwrap().len() > 0);
    }
    let image = png::Decoder::new(BufReader::new(
        fs::File::open(folder.path().join("motion.png")).unwrap(),
    ))
    .read_info()
    .unwrap();
    assert_eq!(image.info().height, 500);
    assert!(image.info().width >= 640);
    let before = fs::read(folder.path().join("summary.json")).unwrap();
    assert!(write_outputs(&input, &repeated, folder.path()).is_err());
    assert!(before == fs::read(folder.path().join("summary.json")).unwrap());
}
