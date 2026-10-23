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

Apply the same path-continuity rule to the fast analytic map, using amplitude (the square root of
tap energy) for the half-maximum condition. The unmodified summed-energy argmax jumped between
microphones and produced a 12.27-tap residual in `drift_50ppm`, seed 1. The prior must still select
a measured local maximum; a unit test confirms that a weak or missing path yields to the global
peak. Both parameters remain configurable.

Rechecks compare delays at the same frame. GCC measures a window centre, while fast feedback
starts at the later lock hop. Retain those two origins and account for their separation with the
estimated rate. Omitting that offset falsely re-locked the 200 ppm case after twelve seconds.
With it, repeated rechecks preserve the lock. Route jumps still require two agreeing unexplained
rechecks. No scenario gate or acceptance threshold was relaxed.

The acoustic tests use PCM and capture timestamps, with no trajectory labels in alignment. They
cover 200 ms latency, both specified drift rates and Bluetooth delay. Fast-map feedback meets the
0.3-tap residual limit after five seconds in the short acoustic runs, including repeated rechecks.
The full 120-second scenarios and gesture gates remain part of the complete engine's validation.
