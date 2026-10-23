# Tuning

M4 is still in progress. These are synthetic PCM measurements, not device performance. No real
recordings have been supplied, and the installed preview still uses the earlier level host.

## Gesture measurements

The original defaults retain a 40 ms complex average and input-coherence mode selection. The
explicit development probe currently matches 0/10 pokes for basic, 200 ppm and PC input across
seeds 1–3. These runs are forty seconds; the complete 120-second drift/PC gates remain pending.

The single-speaker candidate uses `fast_seconds = 0.005` and `infer_single_path = true`. Other
thresholds retain their defaults, including the unchanged motion threshold and relative floor.
Its physical mode comes from measured transfer paths, never scene labels. After meter integration,
the candidate matches 30/30 basic phone pokes across seeds 1–3, with no extras and median
confirmation below 80 ms. The candidate is not the application default.

The full negative runs below used that candidate before meter integration:

| Scenario | Seconds per seed | Seed 1 false pokes | Seed 2 | Seed 3 | Required  |
| -------- | ---------------: | -----------------: | -----: | -----: | --------- |
| Still    |               60 |                  0 |      0 |      0 | 0         |
| Hover    |              300 |                  0 |      8 |      1 | At most 1 |
| Wave     |              120 |                  0 |      0 |      0 | At most 1 |
| Typing   |              300 |                  0 |      0 |      0 | At most 1 |
| Walking  |              300 |                  0 |      0 |      0 | 0         |

Stereo, PC and drifting gesture recall are not sufficient. Shorter floor calibration improved one
stereo seed to 9/10; it did not solve PC input. No gesture acceptance threshold has been relaxed.
The full scenario matrix and live actions must not be accepted from the basic phone result alone.

Stronger ridge, frequency smoothing, full covariance whitening and an overlap-save adaptive
canceller did not meet the gates. Their prototypes were removed. Eight-tap Lanczos interpolation
has measurable high-band phase/gain error, but a counterfactual PC run with zero clock drift also
missed its poke. The renderer remains unchanged; interpolation error does not explain every failure.

## Meter measurements

Forty-second runs, seeds 1–3, excluding the first five seconds:

| Scene           | Measurement              | Result                                        |
| --------------- | ------------------------ | --------------------------------------------- |
| Dull            | Mean high factor         | 0.274                                         |
| Compressed16k   | Mean effective bandwidth | 10,047–10,052 Hz                              |
| Compressed16k   | Reported detail          | 1.7 cm                                        |
| Gappy           | Mean steady factor       | 0.552                                         |
| Blocked capture | Tip within three seconds | `capture_blocked`, score 0                    |
| Silence         | State and tip            | `WaitingForSound`, `nothing_playing`, score 0 |

[Decision 0013](decisions/0013-meter-scenario-expectations.md) explains the compressed-bandwidth
and Dull-tip gate corrections. The specified filter and tip priority remain intact. Factor fitting
against labelled real recordings belongs to M5; no score-to-recall calibration is claimed yet.

Run the component regressions with `cargo test --release -p veyl-dsp -p veyl-sim -p veyl-lab`.
Run `cargo run --release -p veyl-sim --example poke_gates` separately for the current failing
default-gesture development probe. Full lab exports, deterministic replay and performance checks
are the remaining M4 tools for the next tuning pass.
