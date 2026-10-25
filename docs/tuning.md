# Tuning

These are synthetic PCM measurements, not device performance. No real recordings have been
supplied, and the installed preview still uses the earlier level host.

## Current default

`EngineConfig::default()` is the tuned engine of decisions 0020–0024. The CI scenario gates
(`crates/veyl-lab/tests/scenario_gates.rs`, thresholds in
[decision 0023](decisions/0023-tuned-defaults-and-scenario-thresholds.md)) pass on seeds 1–3.
Against the plan's specified thresholds, the probe measures:

| Scenario       | Seed 1        | Seed 2        | Seed 3         | Specified gate |
| -------------- | ------------- | ------------- | -------------- | -------------- |
| Basic          | 10/10, 130 ms | 10/10, 131 ms | 10/10, 127 ms  | fails (delay)  |
| Varied         | 27/30         | 20/30         | 23/30          | fails          |
| Typing + pokes | 9/10          | 10/10         | 10/10, 1 extra | passes         |
| Volume steps   | 7/10, 1 extra | 7/10          | 6/10, 2 extra  | fails          |
| Dropouts       | 10/10         | 8/10          | 9/10           | fails seed 2   |
| PC             | 10/10         | 9/10          | 10/10          | passes         |
| PC Bluetooth   | 10/10         | 10/10         | 8/10           | fails seed 3   |

Still, hover, wave, walking, typing, gappy and blocked stay within their limits; stereo,
compressed and both clock cases match every poke. 10 of 54 cases fail the specified thresholds.

Processing cost on the development desktop ([decision 0024](decisions/0024-processing-cost.md)):
31.4 ms per second of audio for a phone scene and about 47 ms for the stereo PC scene; the
specified configuration costs 14.6 ms. The ignored 60-second performance test passes for the
phone scene (1.84 s) and fails for PC (2.82 s). The older laptop of §6.10 is not measured yet.

## Segment clicks, gaps and deferred pokes

[Decision 0022](decisions/0022-segment-clicks-gaps-and-deferred-pokes.md) adds
`change_in_band`, `change_own_fit`, `change_blank_ratio = 4`, `change_impulse_ratio = 12`,
`change_gap_ms = 12`, `poke_defer_suspension` and `floor_learn_fraction = 0.5` to the
short-change candidate. Full-duration probe replays, seeds 1–3:

| Scenario       | Seed 1        | Seed 2        | Seed 3         | Gate   | Decision 0021 |
| -------------- | ------------- | ------------- | -------------- | ------ | ------------- |
| Basic          | 10/10, 130 ms | 10/10, 131 ms | 10/10, 127 ms  | fails  | 30/30         |
| Varied         | 27/30         | 20/30         | 23/30          | fails  | 55/90         |
| Typing + pokes | 8/10          | 10/10         | 10/10, 1 extra | fails  | 12/30         |
| Volume steps   | 6/10          | 6/10          | 6/10, 2 extra  | fails  | 18/30         |
| 50 ppm         | 10/10         | 10/10         | 10/10          | passes | 30/30         |
| 200 ppm        | 10/10         | 10/10         | 10/10          | passes | 30/30         |
| Dropouts       | 10/10         | 8/10          | 9/10           | fails  | 21/30         |
| Compressed16k  | 10/10         | 10/10         | 10/10          | passes | 30/30         |
| Stereo         | 10/10         | 10/10         | 10/10          | passes | 30/30         |
| PC             | 10/10         | 10/10         | 10/10          | passes | 29/30         |
| PC Bluetooth   | 10/10         | 10/10         | 8/10           | fails  | 24/30         |

Still, hover, wave, walking, typing, gappy and blocked stay at or below their limits on every
seed (typing seed 3 has one false poke, its limit). 11 of 54 cases fail, against 17; three are
basic latency. Measured against the pokes deep and fast enough for Normal's own thresholds,
varied matches 27/28, 20/25 and 21/22.

What the remaining failures show:

- **Volume steps.** After each ±6 dB step the simulated speaker's soft clipping changes the scene:
  the segment gate holds for about a second while its median adapts, then the old background's
  error near the top microphone's direct sound reaches z ≈ 10,000 until it relearns. Pokes
  within about a second of a step are lost.
- **Varied seed 2.** Most misses follow a brief blur during the approach (z below half the on
  threshold for six hops), which ends presence and loses the approach.
- **First pokes.** Several single-seed misses are the first poke at 5 s, during or just after
  settling.

Measured and rejected in this pass, seed by seed in the full matrix: a floor that falls faster
than it rises (time constants 0.3, 1.0 and 1.5 s: 20–26 failing cases, with clock, walking, PC
and typing failures); a floor that follows a tap above threshold for 1–1.5 s (no change: the
disturbance lasts about 0.7 s); re-settling on a field-level change of 2–3 dB (volume steps 2–3
of 10, because the floor then calibrated on an unconverged background); background adaptation
after 12 or 24 held hops (up to 17 false volume-step pokes and 61 false typing pokes); a
farther microphone's exclusion of 14 or 20 taps after its direct sound (more false typing
pokes); and presence hold-overs of 10 or 14 hops (12–13 failing cases).

A known weakness: after a long, still hover, a fast departure keeps the track where the hand
was. `motion_speed` (six seconds still at 32 cm, then straight in and out) puts the strongest
echo on the hand in 99%, 97%, 16% and 3% of moving hops at 8, 30, 50 and 80 cm/s, against 88%,
81%, 35% and 0% for decision 0020's candidate and 100%, 97%, 43% and 17% for decision 0021's.
The background still holds the hovering hand, and the 4 cm track gate keeps the track on that
stale change; a stronger echo outside the gate takes the track only after three hops at twice
its strength. Ratios of 1.25 and 1.5 gained a few percent and added a false typing poke. Without
the gate, 77% and 27%. Pokes from a hover pass their gates because their approach starts from the
hover position. Real recordings decide whether this needs more work.

`processing_cost`, same method as below: specified 15.2 ms, residual 42.0 ms and default 63.0 ms
per second of audio. M4.8's 30 ms budget is not met.

## Short-change candidate

[Decision 0021](decisions/0021-short-change-and-steady-tracking.md) adds `change_window = 1024`,
`change_smooth_bins = 8`, `track_gate_cm = 4` and `impulse_relative` to the residual candidate,
and corrects the distance origin, each microphone's direct region and the poke's approach start
for every configuration. Full-duration probe replays, seeds 1–3 (matched / truth, then extra
detections and median truth-to-detect delay):

| Scenario       | Seed 1        | Seed 2        | Seed 3        | Gate   | Residual (0020) |
| -------------- | ------------- | ------------- | ------------- | ------ | --------------- |
| Basic          | 10/10, 115 ms | 10/10, 115 ms | 10/10, 107 ms | fails  | 30/30           |
| Varied         | 24/30         | 13/30         | 18/30         | fails  | 31/90           |
| Typing + pokes | 4/10          | 3/10          | 5/10          | fails  | 18/30           |
| Volume steps   | 6/10          | 6/10          | 6/10, 2 extra | fails  | 21/30           |
| 50 ppm         | 10/10         | 10/10         | 10/10         | passes | 30/30           |
| 200 ppm        | 10/10         | 10/10         | 10/10         | passes | 30/30           |
| Dropouts       | 7/10          | 6/10          | 8/10          | fails  | 19/30           |
| Compressed16k  | 10/10         | 10/10         | 10/10         | passes | 30/30           |
| Stereo         | 10/10         | 10/10         | 10/10         | passes | 30/30           |
| PC             | 9/10          | 10/10         | 10/10         | passes | 17/30           |
| PC Bluetooth   | 8/10          | 9/10          | 7/10          | fails  | 21/30           |

Still, hover, wave, walking, typing, blocked and gappy produce no detection. No run re-locks,
raises a model jump or a drift warning, or leaves more than 0.031 taps of direct residual after
five seconds. 17 of 54 cases fail, against 18. Basic fails only its 80 ms median delay: its
200 ms retreat covers 4.57 cm of echo distance, so the 2 cm confirmation rise alone takes 87 ms.
The residual candidate's 51–66 ms came from the vanishing hover echo, not the moving hand.

Diagnostics behind the remaining failures:

- **Varied.** 28, 25 and 22 truth pokes per seed meet Normal's depth and speed thresholds; 24,
  12 and 15 of those match. Approaches above about 50 cm/s still blur within the 1024-sample
  segment, and weak turns stay below the turn-strength guard.
- **Typing with pokes.** Each click holds hops while it crosses the whole 2048-sample window.
  In one traced poke, 18 of 62 hops were held and the valid fraction fell to 71%, below 80%,
  although the track followed the hand. Gating on the segment itself is the next candidate.
- **Volume steps.** The misses follow each ±6 dB step by one or two seconds.
- **PC Bluetooth.** The relative gate removes the hi-hat holds, but hi-hat bursts at the 40 cm
  monitor echo can still start a track ahead of the hand.

A hand moving straight toward the phone and back, from `motion_speed` (share of moving hops whose
strongest echo is within 2 cm of the hand):

| Speed   | Residual (0020) | Short change (0021) |
| ------- | --------------: | ------------------: |
| 8 cm/s  |             88% |                100% |
| 30 cm/s |             81% |                 97% |
| 50 cm/s |             35% |                 43% |
| 80 cm/s |              0% |                 17% |

Earlier in the same pass, before own-power whitening and the gate, 512- and 768-sample segments
kept 76% and 73% at 80 cm/s but lowered mean motion at 8 cm/s from 30 to 13 and 24; PC fell to
0/10 at 512. With a 768-sample segment and no track gate, whitening by the long-term spectrum
gave 9 false hover pokes on seed 1, own power over ±2 or ±8 bins 3, and ±24 bins 1. A 2 cm
track gate lost basic pokes (4/10).

`processing_cost` on the development desktop, ten seconds measured after ten of warm-up, five
repetitions: defaults 15.6 ms, residual 42.3 ms and short change 57.6 ms per second of audio.
M4.8's 30 ms budget is not met.

## Residual candidate

[Decision 0020](decisions/0020-residual-motion.md) adds `residual_motion`. The candidate sets it
with `infer_single_path`, `resample_reference`, `floor_relative = 1e-8` and
`impulse_ratio = 12`; every other parameter keeps its default. Full-duration replays through
the lab, seeds 1–3 (matched / truth, plus extra detections):

| Scenario       | Seed 1 | Seed 2 | Seed 3 | Gate   | Earlier 5 ms candidate |
| -------------- | -----: | -----: | -----: | ------ | ---------------------- |
| Basic          |  10/10 |  10/10 |  10/10 | passes | 30/30                  |
| Varied         |  10/30 |   9/30 |  12/30 | fails  | 24/90                  |
| Typing + pokes |   6/10 |   6/10 |   6/10 | fails  | 10/30                  |
| Volume steps   |   7/10 |   7/10 |   7/10 | fails  | 25/30                  |
| 50 ppm         |  10/10 |  10/10 |  10/10 | passes | 30/30                  |
| 200 ppm        |  10/10 |  10/10 |  10/10 | passes | 30/30                  |
| Dropouts       |   8/10 |   5/10 |   6/10 | fails  | 17/30                  |
| Compressed16k  |  10/10 |  10/10 |  10/10 | passes | 21/30                  |
| Stereo         |  10/10 |  10/10 |  10/10 | passes | 0/30                   |
| PC             |   5/10 |   6/10 |   6/10 | fails  | 0/30                   |
| PC Bluetooth   |   7/10 |   8/10 |   6/10 | fails  | 0/30                   |

Negative cases detect 1, 0 and 0 pokes while hovering, and none for still, wave, walking, typing
or blocked capture. Gappy confirms nothing during its gaps. No run raises a model jump, drift
warning or re-lock; the largest direct residual after five seconds is 0.084 taps outside the
gappy scene. Basic median truth-to-detect delays are 54, 77 and 62 ms. 18 of the 54 cases still
fail their gates, against 24 for the earlier candidate. Volume steps lose one poke per seed
compared with the earlier candidate; the other shared cases improve or hold.

Several varied pokes are shallower than the Normal preset's own 3 cm minimum, measured in echo
distance: 28, 25 and 22 of the 30 truth pokes per seed meet its depth and 10 cm/s speed
thresholds. Most remaining misses, however, come from the tracker: a fast approach blurs the
hand's echo within the 43 ms window, presence ends, and the turn arrives without its approach.

Measurements behind the design, from probes of single scenes:

- Stereo mode raised the PC motion floor to about three times that of the same speakers playing
  one channel. Removing clock drift, the diffuse tail or static echoes each lowered the
  remaining floor; the fixed 43 ms window cannot model the 30 ms tail exactly.
- The per-hop gain and sub-sample timing fit raised the PC first-poke motion peak from about 6
  to 10–11. Smoothing the gain instead made no difference.
- With correlated channels, separate per-channel steps left a stereo residual as large as a
  new echo after four seconds; the loaded long-term inverse converges.
- Each choice of clock feedback source was tried on all five clock and geometry cases. Fast
  per-speaker peaks ran away in PC seeds 2–3; peaks of the per-window mean field wandered with
  the music's balance; delay-only feedback diverged. Background peaks plus the hop's delay track
  every case within 2 ppm.
- A gated tracker that coasts through blurred hops recovered varied pokes but produced 26–46
  false typing pokes. A memory of absorbed echoes for the vanished-echo guard lost most dropout
  pokes. Neither is part of the candidate.

An isolated release comparison on the development desktop, twenty seconds of input with ten
measured, five repetitions: the earlier candidate takes a median 31.6 ms per second of audio
and the residual candidate 47.6–49.2 ms. Both exceed the 30 ms target; M4.8 remains open.

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

A separate comparison applies the existing High preset uniformly across selected cases, with
the same signal-processing candidate. Varied matches improve to 19/30, 17/30 and 18/30, while
hover produces 9, 8 and 4 false pokes. Typing with pokes reaches 6/10 for each seed; stereo and
PC still match none. High therefore does not close the accuracy gates and is not the default.
The lab probe's `--sensitivity=high` option records the preset in every measurement.

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
The scenario gates run in `crates/veyl-lab/tests/scenario_gates.rs`. [Full lab exports and sweeps](lab.md) now support the next
tuning pass. The seed-1 forty-second candidate report measures median truth-to-detect delay
57.98 ms, p95 98.25 ms and median confirmation 18.67 ms. Its motion heatmap was inspected.
Performance and allocation checks remain M4.8 work.

## Startup and processing cost

Bounded sparse alignment ([decision 0018](decisions/0018-align-through-bounded-short-gaps.md))
reduces dropout seed 2's first lock from 5.85 to 2.33 seconds. All three dropout seeds now lock
at 2.33 seconds. Recall stays 8/10, 5/10 and 4/10, with zero extra pokes. Other full candidate
counts above are unchanged in the latest 54-case replay. Hop validity and the 300 ms post-gap
suspension still apply; faster startup alone does not establish reliable gestures.

An isolated release comparison on a Ryzen 5 7600X uses identical synthetic 2-reference,
2-microphone input at 48 kHz, ten seconds of warm-up and ten seconds measured. Both configurations
select Mono; five-second alignment rechecks remain enabled. Six clean timings per configuration
measure the complete push/process calls:

| Sampler accumulation   | Median ms per second of audio |       Range |
| ---------------------- | ----------------------------: | ----------: |
| One serial sum         |                         45.81 | 45.03-46.24 |
| Eight independent sums |                         29.32 | 28.82-30.09 |

Both keep the 64-tap kernel and phase table, allocate nothing in warmed push/process calls,
and replay every exposed event and snapshot field exactly within the same build. Reordered
sums differ from the scalar sampler by less than 1e-6 for normalized test input. Independent
64-tap high-band reconstruction stays below 1e-4 relative RMS at 44.1 and 48 kHz. These are
development-desktop measurements, not the specified older laptop. Some repetitions exceed
30 ms, so M4.8's hardware budget remains unverified. See
[decision 0019](decisions/0019-parallel-fractional-sampler-sums.md).

`crates/veyl-sim/tests/engine_properties.rs` checks warmed allocation operations and bitwise
replay for both defaults and the opt-in candidate. It deliberately avoids a wall-clock limit in
shared CI; target-hardware timing remains a separate measurement.
