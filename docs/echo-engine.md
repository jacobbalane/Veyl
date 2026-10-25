# Offline echo processing

The portable engine (`crates/veyl-dsp`) works on actual filtered PCM. It reports hand presence,
distance, pokes and the sound-quality meter. The desktop app's engine host runs it live in phone
and PC mode (decision 0025); the lab runs the same engine offline on simulator scenes and test
recordings. Its tuning so far comes from the simulator; real recordings decide the final values
(see [tuning](tuning.md)).

`SampleRing` stores four seconds per channel by absolute frame number. Missing frames and flagged
boundaries invalidate any window that touches them. Overlap cannot replace previously accepted
audio. Capture-time anchors relate the reference and microphone clocks; network arrival time does
not enter the acoustic estimates. Input i16 samples become float by division by 32768.

`Aligner::try_align` chooses one second of mono reference and a matching microphone span. With
timestamps it searches from 30 ms before simultaneous capture to 300 ms after it, or 500 ms in PC
mode. Without timestamps it searches one second either side of the initial frame-origin
difference. It requires two good GCC-PHAT estimates a second apart and rechecks every five seconds.
Window origins are included when converting correlation results to absolute lag.

The lag search tolerates zero-filled holes covering at most 2% of each input, with no gap
longer than 10 ms. These limits are `align_max_missing_fraction` and `align_max_gap_ms`; zero
disables tolerance. The span must stay inside received, retained audio. Processing windows
still require every frame, and gaps still clear gestures and suspend confirmation for 300 ms.
See [decision 0018](decisions/0018-align-through-bounded-short-gaps.md).

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

The sampler combines eight independent tap sums for contiguous, fully supported input. Partial
support and callbacks retain scalar sampling. The kernel is unchanged; addition order changes
rounding within the tested numerical bound. See
[decision 0019](decisions/0019-parallel-fractional-sampler-sums.md). When the estimated rate
difference spans less than 0.01 frames across the window, as with a phone's own playback copy
and microphone, the sampler is skipped and the fraction becomes a phase ramp. Forward transforms
of real windows run two to a complex FFT, and empty map slots are not rendered (decision 0024).

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
later meter. Enable this gate only after one second of Running. With the opt-in
`impulse_relative`, the history holds the residual divided by the predicted power instead: a
louder moment of the music raises both, while a click raises only the residual.

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

Updating only media volume preserves a pending gesture: direct-path normalization handles that
gain change. Changing source, route, capture or microphone state still clears pending gestures.

Echo distances count from the earliest direct sound at least half as strong, in amplitude, as
the strongest: the nearest speaker and microphone. A farther microphone hears its own direct
sound inside that range, so its direct region (8 taps before, 6 after its strongest background
tap) is left out of its motion energy. A poke's approach starts at the deepest earlier point
that still meets the depth, approach-speed and duration rules, so a slow drift into position
cannot dilute a fast approach.

In residual mode, `change_window` measures change over that many microphone samples at the
centre of the window. Each hop predicts those samples from the background by exact linear
convolution, correlates what remains with the mean reference, and renders the change and
current-field maps with an analytic inverse FFT of `EngineConfig::change_size()` points. With
`change_smooth_bins`, change is whitened by the segment's own power smoothed across that many
bins on each side. While a hand is present, `track_gate_cm` limits the peak search to that
distance around its track. See [decision 0021](decisions/0021-short-change-and-steady-tracking.md).

The default configuration is the tuned engine of decisions 0020–0023; `EngineConfig::specified()`
keeps the plan's §6 values, including the 40 ms fast average without residual motion, for
component tests and comparisons. `crates/veyl-lab/tests/scenario_gates.rs` replays every
simulator scenario for seeds 1–3 through the lab and checks the gesture gates, with the
thresholds decision 0023 changed. The [lab](lab.md) exports full analysis, motion plots and
parameter sweeps for continued tuning.

Run the required portable suite with:

```powershell
cargo test --release -p veyl-dsp -p veyl-sim -p veyl-lab
```

Unit tests compare analytic maps against known signed-delay responses, recover known mono and
stereo echo paths at both supported rates, check the fractional phase direction, exercise mode
switching and verify impulse hold/recovery. Simulator tests drive the same alignment and feedback
code from PCM and timestamps, without using movement labels.
