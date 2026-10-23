# Offline analysis

The lab reads a selected local recording or a generated scene. It replays filtered audio through a
fresh engine and writes measurements locally. It never captures a device, uploads data or creates
another audio copy during analysis. Gesture accuracy is still under development.

Generate a scene in a new folder and analyze it:

```powershell
cargo run --release -p veyl-lab -- sim poke_basic --seed 1 --out .cache/lab/basic
cargo run --release -p veyl-lab -- analyze .cache/lab/basic --out .cache/lab/basic-report
```

The default report honestly shows the current defaults' missed gestures. For the experimental
single-speaker phone candidate, save this profile as an ignored `params.toml`:

```toml
fast_seconds = 0.005
infer_single_path = true
resample_reference = true
```

Then pass `--config params.toml`. A profile uses the engine's defaults for omitted fields. Without
a profile, mode and privacy cutoff come from the recording; PC input defaults to Far range. In a
PC-specific profile, set `pc_mode = true` and `range_max_cm = 55.0` explicitly. Unknown fields,
non-finite values and unsafe dimensions are rejected.

The output contains:

| File           | Contents                                                                                                         |
| -------------- | ---------------------------------------------------------------------------------------------------------------- |
| `summary.json` | Stream/gap statistics, effective configuration, alignment, warnings, mean meter factors and gesture measurements |
| `features.csv` | Every processed hop: state, validity, motion, peak/filtered distance, presence, floor, suspension and drift      |
| `meter.csv`    | Meter readings at 100 ms intervals, plus source-status updates                                                   |
| `pokes.csv`    | Estimated turn, confirmation frame, distance, depth, confidence and confirmation delay                           |
| `motion.png`   | Time × echo distance, 0–30 dB motion, labels and detected pokes                                                  |

Existing reports are preserved. Choose a new `--out` folder for another run. Omitting `--out` puts
the reports beside the input files. Means include startup; the CSVs allow a separate settled-only
comparison. Offline analysis starts a new engine, so a recording begun during an existing live
session does not include that session's earlier learned background.

Truth labels measure recall and actual simulator delay. Human Space marks have a wider matching
window because they include reaction time; their offset is reported separately from confirmation
delay. A real recording without marks has unknown recall, precision and false-poke rate. For a
known still control with no expected pokes, set `"annotations_complete": true` inside `meta.json`'s
`device` object. Set it false for partial labels. See
[decision 0014](decisions/0014-offline-analysis-measurements.md) for the matching rules.

Compare parameters across one or more recordings:

```powershell
cargo run --release -p veyl-lab -- sweep .cache/lab/basic --set fast_seconds=0.005,0.04 --set infer_single_path=true,false --out .cache/lab/sweep.csv
```

The CSV has one row per combination, pooling labels, detections, duration and raw delay samples
across the inputs. The terminal lists the best F1 results with at most 0.2 false pokes per minute.
Unlabelled real inputs cannot establish that rate and are not ranked. The output path must be new.
Sweeps support up to 16 distinct parameters, 32 values per parameter and 4,096 combinations.

The phone candidate's basic and clock-drift results are regression checks, not full acceptance.
Hover, stereo and PC gesture gates remain unresolved in [tuning](tuning.md). No R1–R10 device
recordings have been supplied yet.

Run the current candidate's gesture checks without writing audio or analysis files:

```powershell
cargo run --release -p veyl-lab --example candidate_gestures -- 1
```

Repeat with seeds 2 and 3. To select cases, append names after the seed. The probe runs the full
specified durations, prints one JSON measurement per case and exits unsuccessfully on failed
gesture checks. It is a development probe; the complete CI gates, including meter checks and
stereo-mode selection, remain separate. The current candidate fails several cases.

Use `--sensitivity=low`, `--sensitivity=normal` or `--sensitivity=high` after the seed to compare
an existing preset. One preset applies to every case in that run and is included in each JSON
measurement. The default remains Normal. No preset is selected from truth labels.
