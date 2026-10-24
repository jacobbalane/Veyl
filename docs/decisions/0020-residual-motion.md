# 0020: Measure motion from the background residual

Date: 2026-10-24

The 5 ms candidate measures change as the difference between a per-hop transfer estimate and
the background. Each hop divides by that window's own reference spectrum, and Stereo inverts a
2×2 matrix from two overlapping windows. Correlated left and right channels make that inverse
noisy: in the PC scene the motion floor was about three times the Mono-content floor, and the
hand never rose above it (peak motion 1–2 against an on threshold of 8). Stereo speakers and
both PC scenes matched no pokes. Per-speaker gains and direct peaks were also ambiguous:
PC runs raised a model jump every 300 ms, and the clock tracker could follow a different
speaker's direct sound and run away to −2,700 ppm.

Add an opt-in `residual_motion` mode with these parts:

- **Residual.** Compare each microphone with the background's prediction, matched by one complex
  gain per microphone measured against the whole predicted field. Remove the per-hop
  least-squares fit of a gain and a sub-sample delay of that field: neither is a new echo.
- **Change map.** Average the residual's cross spectrum with the mean of the reference channels
  over `motion_seconds` (5 ms). Divide by that reference's long-term spectrum
  (`reference_power_seconds`, 0.5 s) scaled to this hop's level. A hand reflects every
  speaker, so one projection keeps its echo without a per-hop inverse. The vanished-echo guard
  uses the same residual divided by this window's own power, so a vanished echo cancels.
- **Background learning.** Mono uses normalized LMS steps. Stereo uses the long-term 2×2
  covariance inverse with 10% diagonal loading, so the weakly excited left-minus-right direction
  converges without the per-hop noise. Settling takes fast steps (`residual_settle_seconds`,
  50 ms) at unit gain; dividing by a gain measured against a half-learned field fed back and
  diverged. Three model jumps in a row restart the background instead of holding it forever.
- **Motion floor.** Settling learns the floor from the recent mean change energy.
- **Clock feedback.** Each microphone's background, seen through the mean reference, keeps a
  stable direct peak. Its position plus the hop's whole-field delay, averaged over microphones
  from where each started, feeds the drift tracker. Feedback starts 16 hops after the
  background starts, and drift warnings wait until settling ends.
- **Impulse gate.** The gate predicts the microphone with the gain-matched background, as the
  residual does. Without the gain, a strong Stereo hand echo held seven hops after its turn and
  the poke failed the valid-hop rule.

The candidate also sets `infer_single_path`, `resample_reference`, `floor_relative = 1e-8` and
`impulse_ratio = 12`. The lower floor bound matters in PC mode: the measured floor there is near
1e-7 of the direct energy, so the specified 1e-6 bound capped the hand's motion score below the
on threshold. With the gain-matched gate, the specified ratio of 6 also held hops at strong
turns and raised basic delay to 81 ms on seed 1; typing clicks still exceed 12 by far, and
typing produces no pokes. Defaults do not change, and no gesture gate changes.

Across the 18 full-duration probe scenarios and seeds 1–3, 18 of 54 cases fail, against 24 for
the earlier candidate. Stereo, compressed and both clock cases match all 120 of their pokes
without extras; PC matches 17 of 30 and PC Bluetooth 21 of 30, from none. Volume steps lose one poke per
seed. Varied, typing-with-pokes, dropout and both PC gates still fail, so M4.5 stays open; see
[tuning](../tuning.md). The candidate costs about 48 ms per second of audio on the development
desktop against 31.6 ms for the earlier one; both exceed the 30 ms target.

Rejected along the way: a separate normalization per reference channel without any inverse
(converged too slowly in the difference direction), the long-term inverse for the change map
itself (three times the noise), smoothing the gain (no effect), wider band edges (no floor
change), a memory of absorbed echoes for the vanished-echo guard (lost most dropout pokes),
a gated distance tracker (dozens of false typing pokes), and clock feedback from the whole-field
delay alone (too noisy per hop; the tracker diverged). Their measurements are in
[tuning](../tuning.md).
