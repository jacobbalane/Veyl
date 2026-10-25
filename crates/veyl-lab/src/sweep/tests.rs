use super::*;

#[test]
fn parameter_grid_rejects_unknown_non_numeric_repeated_and_unbounded_settings() {
    for value in ["unknown=1", "z_on=NaN", "z_on=\"8\"", "z_on=", "z_on=1e500"] {
        assert!(parameter(value).is_err());
    }
    let duplicate = [
        parameter("z_on=5,8").unwrap(),
        parameter("z_on=12").unwrap(),
    ];
    assert!(combinations(&duplicate).is_err());
    let large = [
        parameter("z_on=1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16").unwrap(),
        parameter("poke_in_cm=1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16").unwrap(),
        parameter("hop=1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16").unwrap(),
        parameter("window=512,1024").unwrap(),
    ];
    assert!(combinations(&large).is_err());
    let grid = [
        parameter("z_on=5,8,12").unwrap(),
        parameter("infer_single_path=true,false").unwrap(),
    ];
    assert_eq!(combinations(&grid).unwrap().len(), 6);
}

#[test]
fn sweep_pools_label_counts_and_raw_latencies_ranks_the_candidate_and_preserves_existing_files() {
    let root = tempfile::tempdir().unwrap();
    let directories: Vec<_> = ["poke_basic", "still"]
        .iter()
        .map(|name| {
            let scene = veyl_sim::scenarios::scene(name, 1, Some(8.0)).unwrap();
            let input = Input::generated(veyl_sim::scene::render(&scene).unwrap());
            let directory = root.path().join(name);
            input.save_generated(&directory).unwrap();
            directory
        })
        .collect();
    let parameters = [
        // The default engine finds the poke; the specified 40 ms average without residual motion
        // does not.
        parameter("residual_motion=true,false").unwrap(),
        parameter("infer_single_path=true").unwrap(),
    ];
    let output = root.path().join("sweep.csv");
    let result = run(&directories, &parameters, &output).unwrap();
    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.rows[0].metrics.labels, 1);
    assert_eq!(result.rows[0].metrics.matched, 1);
    assert_eq!(result.rows[0].metrics.recall, Some(1.0));
    assert_eq!(result.rows[0].metrics.precision, Some(1.0));
    assert_eq!(result.rows[0].metrics.false_pokes_per_minute, Some(0.0));
    assert!(result.rows[0].metrics.latency_median_ms.unwrap() < 200.0);
    assert_eq!(result.rows[1].metrics.recall, Some(0.0));
    assert_eq!(result.best[0], 0);
    let before = fs::read(&output).unwrap();
    assert_eq!(std::str::from_utf8(&before).unwrap().lines().count(), 3);
    assert!(run(&directories, &parameters, &output).is_err());
    assert!(before == fs::read(&output).unwrap());
    let invalid = root.path().join("invalid.csv");
    assert!(run(&directories, &[parameter("window=0").unwrap()], &invalid).is_err());
    assert!(!invalid.exists());
}
