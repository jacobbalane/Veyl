# 0021: Measure change over a short segment and keep the track on the hand

Date: 2026-10-25

Hop-by-hop traces against the simulator's true echo distance showed that the residual candidate
(decision 0020) rarely saw a moving hand:

- **Motion cancels over the window.** A hand approaching at 50 cm/s shortens its echo by several
  taps within the 43 ms window, and the 8–20 kHz phases cancel. With a hand moving straight in
  and out, the strongest echo sat on the hand in 81% of hops at 30 cm/s, 35% at 50 cm/s and none
  at 80 cm/s. Basic pokes were confirmed from the hover echo that vanished and returned, which is
  also why their median delay was near 55 ms.
- **The impulse gate held approaches.** It compares the residual with its median. A hi-hat
  raises the model's error with the music, so 10–12% of hops were held, mostly in the 0.4 s
  before each turn: 93% of held hops in PC Bluetooth, where a hi-hat reaches the microphone as
  each approach starts.
- **Distances counted from the farther microphone.** The two simulated direct sounds are equally
  strong, so the origin could fall on the top microphone, 5.6 cm short of the truth; low pokes
  then read below the 4 cm range.
- **A slow drift diluted the approach.** The farthest point in 900 ms often lay before a slow
  move into position, so the averaged approach speed fell below 10 cm/s.

Opt-in parameters:

- `change_window` (1024): in residual mode, predict the central 1024 microphone samples from the
  background by exact linear convolution and correlate the remainder with the mean reference.
  Change and the vanished-echo guard come from that segment; the background, gain, timing fit and
  clock feedback keep the whole window.
- `change_smooth_bins` (8): whiten that change by the segment's own power smoothed over ±8 bins,
  so a transient's spectral shape does not inflate the error at strong static echoes.
- `track_gate_cm` (4): while a hand is present, seek its echo within 4 cm of its track.
- `impulse_relative`: hold for the residual relative to the predicted power, which a louder
  moment of the music leaves unchanged and a click raises.

Corrections for every configuration: echo distances count from the earliest direct sound at
least half as strong in amplitude as the strongest; each microphone's own direct region (8 taps
before, 6 after) is left out of its motion energy; and a poke's approach starts at the deepest
earlier point that meets the depth, speed and duration rules. The last replaces §6.7's choice of
the farthest point; every other detector rule and threshold is unchanged.

Across the 18 full-duration probe scenarios and seeds 1–3, PC now passes every seed (29/30) and
stereo, compressed and both clock cases keep all their pokes with no re-lock, model jump or drift
warning; negative scenes produce no false poke. Varied rises from 31 to 55 of 90, PC Bluetooth
from 21 to 24 of 30 and dropouts from 19 to 21. Typing with pokes falls from 18 to 12 of 30:
each click holds hops while it crosses the whole window, and holds cut the valid-hop fraction
below 80%. Volume steps fall from 21 to 18 of 30. 17 of 54 cases fail, against 18.

Three of those are basic latency. Its retreat moves 4.57 cm of echo distance in 200 ms, so a
2 cm rise needs at least 87 ms after the true turn before any measurement delay; the 80 ms
median gate cannot be met by a detector that follows the hand. This record does not change that
gate; it needs the maintainer's decision. Defaults do not change.

The candidate costs a median 57.6 ms per second of audio on the development desktop, against
42.3 ms for decision 0020's; the 30 ms target is further away.

Rejected: shorter segments (at 8 cm/s, 512 and 256 samples lowered mean motion from 30 to 13
and 8, and PC dropped to 0/10 at 512);
taking the stronger of the whole-window and segment maps (the whole window's approximate
convolution leaves transient errors near static echoes, and Bluetooth fell to 1/10); a 2 cm gate
(basic fell to 4/10); and an impulse gate on the residual left unexplained by the taps (with one
window, clicks scored 0.3–0.6 unexplained, like hands). See [tuning](../tuning.md).
