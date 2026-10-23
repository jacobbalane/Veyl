# 0019: Shorten the fractional sampler's dependency chain

Date: 2026-10-24

The phone candidate costs about 46 ms per second of audio on the development desktop.
Controlled comparisons identify reference resampling as the main increase. Merely removing
per-tap boundary checks does not materially improve it: the single floating-point sum still
depends on every preceding tap.

For contiguous input with complete interpolation support, accumulate eight independent tap
sums and combine them at the end. Keep the same 64-tap kernel, phase table and interpolation.
Handle remaining taps explicitly for other supported even tap counts. Keep the scalar path
for partial support and callback sampling. Use safe Rust without architecture-specific flags.

Changing addition order changes rounding. Independent checks over eight tap counts measure
less than 1e-6 difference for normalized input; analytic high-band reconstruction remains
below 1e-4 relative RMS at both supported rates. Edges retain scalar behavior. The same build
still replays input bit for bit, including every exposed event and snapshot field.

An isolated release comparison on the Ryzen 5 7600X measures a median 45.81 to 29.32 ms per
second of audio, including alignment rechecks. Some repetitions exceed 30 ms; this does not
verify the required older laptop or complete M4.8. Whole-engine accuracy and allocation
regressions remain required before adoption.
