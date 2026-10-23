# 0015: Sample the completed acoustic signal on the microphone clock

Date: 2026-10-23

The original scene renderer applied its eight-tap fractional-delay kernel separately to every
path while advancing the microphone clock. At 20 kHz, that kernel's gain depends strongly on
fractional phase. Clock drift therefore made even a stationary direct return change amplitude.
This is a numerical artifact, independent of gesture labels or detector thresholds.

Render the acoustic mixture and room noise on the nominal source clock first. Convert that
completed signal to the microphone clock with a 64-tap Kaiser-windowed sinc, using 1,025 phases
and linear interpolation between phases. Add microphone self-noise on the microphone clock,
then clip, filter and quantize as before. Zero-clock scenes bypass conversion. Acoustic geometry,
reflection strengths, source sound, clocks, trajectories and labels keep their specified values.

A regression estimates a stationary 20 kHz return's sinusoidal amplitude in five-millisecond
windows. At both drift signs (200 ppm), ripple must stay below 0.02 dB; measured ripple is
0.0026–0.0031 dB. Separate analytic tests check high-band interpolation, DC and integer samples.
Existing alignment, meter and phone-poke regressions must also pass. This correction changes
generated waveforms for nonzero clocks; regenerate those fixtures before comparing profiles.
It does not establish gesture recall, and none of the gesture acceptance gates change.
