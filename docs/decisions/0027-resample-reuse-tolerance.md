# 0027: Reuse resampled reference samples within 1e-3 frames

Date: 2026-10-26

Decision 0024 reuses the reference samples the previous hop resampled while both hops' delay
models agree within 1e-4 frames. Timing each stage of the stereo PC scene showed the reference
window still costing 11 ms per second of audio, against 2 ms for a phone scene: with clocks that
drift (150 ppm in the PC scene), the drift estimate moves more than 1e-4 frames between hops, so
every hop resampled both channels with the 64-tap sampler.

The tolerance is now 1e-3 frames, a phase error of about 3e-3 radians at 20 kHz, far below the
noise of a single hop's measurement. On the 54-case probe (every scenario, seeds 1–3) no result
got worse: one PC case found 10 of 10 pokes instead of 9, and median delays moved by at most
1.1 ms. The reference window now costs 4 ms per second in PC mode. The PC scene's 60 seconds
take 2.40 s instead of 2.82 s on the development desktop; the phone scene is unchanged at 1.81 s.

The PC scene still misses the 2-second performance test on this machine. The rest of its cost is
the short change segment (14 ms per second, as in phone mode), the background residual (7 ms) and
the fast stereo estimate (8 ms), which still decides the reference mode and single-path
inference. Cutting those changes what is measured, so it waits for the target laptop's numbers
(M5.5) and real recordings.
