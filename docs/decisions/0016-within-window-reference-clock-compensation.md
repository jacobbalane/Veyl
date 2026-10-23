# 0016: Compensate clock rate inside each reference window

Date: 2026-10-23

The original engine selects the integer lag and applies one fractional-delay phase ramp per FFT
window. At 200 ppm, the relative clock position changes by 0.4096 samples across a 2,048-frame
window. The direct peak can meet the alignment tolerance while this changing phase still masks
the much weaker hand return. After correcting the simulator's independent interpolation artifact
(decision 0015), the 5 ms phone candidate still detected zero of ten drifting pokes in a
forty-second comparison.

Add opt-in `resample_reference`. Use the aligner's estimated phase and rate to sample the
reference on the microphone clock before windowing. The predicted phase belongs to the hop's
last frame; earlier samples use the corresponding linear rate correction. Sampling uses the
64-tap phase-interpolated sinc. Integer origins remain integer; floating-point positions are
relative to the copied window. Preallocated scratch includes the interpolation support. Missing
support invalidates the hop, and unavailable future reference postpones it without advancing the
tracker. The estimator receives zero additional phase because compensation is already applied.

The candidate (`fast_seconds = 0.005`, `infer_single_path = true`, `resample_reference = true`)
recovers all 70 truth pokes, with no extras, in full 120-second 50/200 ppm runs for seeds 1–3 and
one reversed-clock run. Maximum direct residual after five seconds is 0.02085 taps. Tests also
require no re-lock or drift warning and median truth-to-detect delay at most 80 ms. Signed-lag
and gap tests exercise both compensation paths at 44.1 and 48 kHz. This remains opt-in: PC/stereo
recall and hover false-poke gates are unfinished. Defaults and accuracy requirements do not change.
