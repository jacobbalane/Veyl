# 0011: Keep a consistent acoustic path while clocks drift

Date: 2026-10-23

The initial one-second GCC-PHAT estimates failed to lock the simulated 200 ppm case with two
microphones. Its two equally strong direct paths exchanged rank: estimates were 1919.82 and
1925.96 frames, with timestamp offsets −4.8 and −14.4. Their clock-corrected difference was
15.74 frames, matching the separation of the two microphone paths. Both PSRs exceeded 60.

Compare startup estimates after subtracting the timestamp-observed clock offset at each window's
centre. The first estimate still chooses the global PHAT maximum. For later estimates, prefer a
measured local maximum within the unchanged two-frame agreement tolerance of the predicted path,
provided its amplitude is at least half the global maximum. Compute PSR against the entire allowed
search range, excluding the usual forty-frame peak neighbourhood. If no such peak exists, use the
global maximum. A prior cannot create a peak or bypass the PSR or level tests.

Rechecks use the drift tracker's current delay prediction, so explained drift does not trigger a
re-lock. Route jumps still require two agreeing unexplained rechecks. The peak ratio is a validated
configuration parameter, initially 0.5. No scenario gate or acceptance threshold was relaxed.

The acoustic tests use PCM and capture timestamps, with no trajectory labels in alignment. They
cover 200 ms latency, both specified drift rates and Bluetooth delay. Fast-map feedback and the
full drift residual/gesture gates still require the next echo-estimation stages.
