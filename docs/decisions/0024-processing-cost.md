# 0024: Spend the processing budget where it measures something

Date: 2026-10-25

The tuned default (decision 0023) cost 63 ms per second of audio on the development desktop,
against §6.10's 30 ms on an older laptop. Timing each stage showed most of it in work that
measured nothing new:

- **Repeated transforms.** About 33 FFTs of 2,048 points ran per hop. Residual mode never reads
  the fast estimate's maps, and Mono mode leaves half the map slots empty.
- **Repeated smoothing.** Whitening by the segment's smoothed power recomputed seventeen
  magnitudes per bin, about 7.5 ms per second alone.
- **Unread spectra.** With short change measurements, the whole-window residual still formed
  change and current spectra that nothing used.
- **Unneeded resampling.** The 64-tap sampler resampled both reference channels over the whole
  window every hop, even when the clocks agree, as a phone's own playback copy and microphone
  usually do, and even though consecutive windows share seven eighths of their samples.

Changes, none of which alters what is measured beyond rounding:

- Real windows are transformed two to a complex FFT and separated by conjugate symmetry, in the
  estimator and in the short path; two microphones' real residuals share one inverse transform.
- Empty map slots are not rendered; residual mode skips the fast maps; the short path's own
  segment, not the whole window, forms change and current.
- Smoothing uses a running sum of magnitudes; phase ramps rotate per bin instead of evaluating a
  sine and cosine each; coherence is not updated once a single speaker path is established.
- The sampler is skipped while the rate difference spans less than 0.01 frames across the window;
  the fraction then becomes a phase ramp, as in the specified path. Otherwise samples the previous
  hop resampled are reused while both hops' delay models agree within 1e-4 frames across them,
  about 2e-4 radians at 20 kHz.

The 54-case probe gives identical counts before and after. On the development desktop the
default now costs 31.4 ms per second for a phone scene and about 47 ms for the stereo PC scene,
whose two speakers keep both reference paths in every stage. The new ignored performance test
(60 s in under 2 s) passes for the phone scene (1.84 s) and fails for PC (2.82 s).

Rejected: a 512-sample hop (half the cost, but 19 failing cases, including clocks, PC and
Bluetooth) and a 768-sample segment, which fits a 1024-point transform (short path 12.3 to 8.2 ms
per second, but PC and Bluetooth recall fell to 4–8 and 2–6 of 10). The older laptop budget is
not verified; M5.5 measures it on target hardware.
