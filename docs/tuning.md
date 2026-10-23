# Tuning

M4 is still in progress. These are synthetic PCM measurements, not device performance. No real
recordings have been supplied, and the installed preview still uses the earlier level host.

## Gesture measurements

The original defaults retain a 40 ms complex average and input-coherence mode selection. The
last explicit default development probe matched 0/10 pokes for basic, 200 ppm and PC input across
seeds 1–3. Those forty-second measurements preceded the clock-renderer correction below.

The single-speaker candidate uses `fast_seconds = 0.005`, `infer_single_path = true` and
`resample_reference = true`. Other
thresholds retain their defaults, including the unchanged motion threshold and relative floor.
Its physical mode comes from measured transfer paths, never scene labels. After meter integration,
the candidate matches 30/30 basic phone pokes across seeds 1–3, with no extras and median
confirmation below 80 ms. The candidate is not the application default.

Full-duration lab replays with source control metadata and the turn-strength guard
([decision 0017](decisions/0017-require-motion-at-the-poke-turn.md)) measure:

| Scenario | Seconds per seed | Seed 1 false pokes | Seed 2 | Seed 3 | Required  |
| -------- | ---------------: | -----------------: | -----: | -----: | --------- |
| Still    |               60 |                  0 |      0 |      0 | 0         |
| Hover    |              300 |                  0 |      2 |      0 | At most 1 |
| Wave     |              120 |                  0 |      0 |      0 | At most 1 |
| Typing   |              300 |                  0 |      0 |      0 | At most 1 |
| Walking  |              300 |                  0 |      0 |      0 | 0         |

Stereo and PC gesture recall are not sufficient. Shorter floor calibration improved one
stereo seed to 9/10; it did not solve PC input. No gesture acceptance threshold has been relaxed.
The full scenario matrix and live actions must not be accepted from the basic phone result alone.

| Scenario       | Seconds per seed | Seed 1 matches / truth | Seed 2 | Seed 3 | False pokes, seeds 1 / 2 / 3 |
| -------------- | ---------------: | ---------------------: | -----: | -----: | ---------------------------: |
| Basic          |               40 |                  10/10 |  10/10 |  10/10 |                    0 / 0 / 0 |
| Varied         |              100 |                   9/30 |   7/30 |   8/30 |                    2 / 0 / 0 |
| Typing + pokes |               40 |                   2/10 |   3/10 |   5/10 |                    0 / 0 / 0 |
| Volume steps   |               40 |                   8/10 |   9/10 |   8/10 |                    0 / 0 / 1 |
| 50 ppm         |              120 |                  10/10 |  10/10 |  10/10 |                    0 / 0 / 0 |
| 200 ppm        |              120 |                  10/10 |  10/10 |  10/10 |                    0 / 0 / 0 |
| Dropouts       |               40 |                   8/10 |   5/10 |   4/10 |                    0 / 1 / 0 |
| Compressed16k  |               40 |                   7/10 |   7/10 |   7/10 |                    0 / 0 / 0 |
| Stereo         |               40 |                   0/10 |   0/10 |   0/10 |                    0 / 0 / 0 |
| PC             |              120 |                   0/10 |   0/10 |   0/10 |                    0 / 0 / 0 |
| PC Bluetooth   |              120 |                   0/10 |   0/10 |   0/10 |                    0 / 0 / 0 |

Basic median truth-to-detect delays are 57.98, 53.98 and 60.65 ms. Gappy confirms 0, 1 and 0
pokes, with none during its silence intervals; it has no recall gate. Blocked confirms none.
These measurements expose failed gates instead of treating component-test success as acceptance.
The candidate remains opt-in. A seed-2 sweep of five relative-floor values from 1e−6 to 1e−8
does not improve the seven varied matches and increases false detections. Nine window/time
combinations (512/1024/2048 frames and 5/10/20 ms) also remain below the varied recall gate.

Stronger ridge, frequency smoothing, full covariance whitening and an overlap-save adaptive
canceller did not meet the gates. Their prototypes were removed. A per-pair normalization
experiment improved a controlled gain test but did not improve hover or PC detection; it was
also removed. A counterfactual PC run with zero clock drift missed its poke, so clock correction
does not explain every failure.

## Clock compensation

[Decision 0015](decisions/0015-microphone-clock-conversion.md) corrects the renderer's numerical
amplitude modulation of stationary paths under clock drift. The 20 kHz regression measures
0.0026–0.0031 dB ripple at ±200 ppm. Geometry, reflection strengths and labels keep their values.
Regenerate drifting/PC fixtures made with the older renderer.

The phone candidate additionally sets `resample_reference = true`, compensating the estimated
clock rate inside each window. With the corrected renderer, a forty-second 200 ppm comparison
detects 10/10 pokes with this option and 0/10 without it. The full candidate regression measures:

| Clock    | Seeds | Seconds per run | Matches | Extras | Maximum direct residual after 5 s |
| -------- | ----- | --------------: | ------: | -----: | --------------------------------: |
| 50 ppm   | 1–3   |             120 |   30/30 |      0 |                      0.02085 taps |
| 200 ppm  | 1–3   |             120 |   30/30 |      0 |                      0.01671 taps |
| −200 ppm | 1     |             120 |   10/10 |      0 |                      0.01719 taps |

All seven runs retain their lock, emit no drift warning and meet an 80 ms median
truth-to-detect limit. [Decision 0016](decisions/0016-within-window-reference-clock-compensation.md)
records the implementation. With the turn-strength guard, hover seed 2 produces two false pokes in 300 seconds;
PC and stereo recall remain insufficient. These options remain opt-in until the full matrix passes.

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
default-gesture development probe. [Full lab exports and sweeps](lab.md) now support the next
tuning pass. The seed-1 forty-second candidate report measures median truth-to-detect delay
57.98 ms, p95 98.25 ms and median confirmation 18.67 ms. Its motion heatmap was inspected.
Performance and allocation checks remain M4.8 work.
