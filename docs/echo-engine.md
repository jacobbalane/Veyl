# Offline echo processing

M4 builds the portable engine in stages. Alignment and echo estimation operate on actual filtered
PCM; the application still uses the streaming/level host until M5 connects the complete engine.
The offline pipeline reports hand presence, distance, candidate pokes and the sound-quality meter.
Complete gesture accuracy is still under development; no live gesture behavior is connected yet.

`SampleRing` stores four seconds per channel by absolute frame number. Missing frames and flagged
boundaries invalidate any window that touches them. Overlap cannot replace previously accepted
audio. Capture-time anchors relate the reference and microphone clocks; network arrival time does
not enter the acoustic estimates. Input i16 samples become float by division by 32768.

`Aligner::try_align` chooses one second of mono reference and a matching microphone span. With
timestamps it searches from 30 ms before simultaneous capture to 300 ms after it, or 500 ms in PC
mode. Without timestamps it searches one second either side of the initial frame-origin
difference. It requires two good GCC-PHAT estimates a second apart and rechecks every five seconds.
Window origins are included when converting correlation results to absolute lag.

`Aligner::begin_hop` predicts clock drift and folds whole frames into the lag. Its returned fraction
delays the reference spectra with a phase ramp. After echo estimation, `observe_fast` updates the
two-state delay/rate filter from the direct path; the caller skips feedback on invalid or impulse
hops. [Decision 0011](decisions/0011-clock-aware-alignment.md) explains path continuity and how
rechecks compare estimates made at different frames. Drift itself does not reset the echo model.

The opt-in `resample_reference` path applies the predicted phase and rate within the reference
window before the FFT, using a 64-tap phase-interpolated sinc. It copies and validates the full
interpolation support. Missing support invalidates the hop; future reference delays processing
without advancing alignment. Integer frame origins stay integer, and sampling positions are
relative. The estimator then applies no additional phase ramp. See
[decision 0016](decisions/0016-within-window-reference-clock-compensation.md).

`EchoEstimator` receives channel-major windows: N samples for reference channel 0, then channel 1;
the microphone uses the same layout. Defaults are a periodic 2048-frame Hann window and a
256-frame hop. Fast spectral averages use a 40 ms time constant. Correlated stereo uses a mono
mean; distinct channels use a regularized 2×2 Hermitian inverse. Coherence thresholds of 0.95 and
0.98 provide hysteresis.

Transfer functions and maps have a fixed pair layout: microphone first, then reference, with
`format.ref_channels` slots per microphone. An inactive reference slot is zero in mono mode.
Each transfer has N/2+1 complex bins; each map contains taps −P through K−1. A raised-cosine band
mask and a smoothed reference-activity mask weight the positive frequencies. The complex inverse
FFT makes an analytic map, corrected for the Hann window's autocorrelation at each tap.

The caller supplies the background transfer functions for microphone prediction. A bounded
two-second residual history identifies impulses above six times its median. An impulse holds the
averages/maps for that hop and the next two; the frame also reports the gated fraction for the
later meter. Enable this gate only after one second of Running.

Construction plans FFTs and allocates working buffers. Processing reuses them, including the
median scratch. `EngineConfig::from_toml` supplies defaults, rejects unknown fields and validates
dimensions and finite values. Background learning and motion features now run in the portable
Engine. The caller supplies a reusable event buffer; per-hop feature events support later CSV
analysis. `snapshot` consumes the maximum motion row since the previous snapshot, without altering
processing events. Gap windows hold features, and valid windows resume learning while detection
stays suspended. Background normalization rejects large gain/phase jumps. Presence slows learning
for three seconds, then stationary objects fade normally. A gated noise floor, peakiness and
three/six-hop hysteresis produce Hand changes; distance uses a three-value median and 15 ms
smoothing. The frame-indexed poke detector confirms an approach and retreat, and discards
trajectories on a source/preset change, pause, gap or impossible distance jump. The meter gates
confirmation below score 15 and during unsafe or silent source conditions. New engines start with
capture off: callers must use `set_source_status` for their active source. `Meter` events arrive
every 100 ms even when the engine is waiting for sound. Snapshot includes the latest reading.

The physical-path inference experiment is opt-in through TOML. Coherence smoothing is independent
of fast transfer smoothing. The default fast time remains 40 ms; a 5 ms candidate detects the
basic single-speaker phone gestures but still fails other required scenarios. See
[decision 0012](decisions/0012-motion-and-poke-guards.md) and the explicit failing development probe:

```powershell
cargo run --release -p veyl-sim --example poke_gates
```

Passing the component suite below does not close those remaining gesture acceptance gates.
The [lab](lab.md) now exports full analysis, motion plots and parameter sweeps for continued tuning.

Run the required portable suite with:

```powershell
cargo test --release -p veyl-dsp -p veyl-sim -p veyl-lab
```

Unit tests compare analytic maps against known signed-delay responses, recover known mono and
stereo echo paths at both supported rates, check the fractional phase direction, exercise mode
switching and verify impulse hold/recovery. Simulator tests drive the same alignment and feedback
code from PCM and timestamps, without using movement labels.
