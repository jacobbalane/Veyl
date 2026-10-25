# 0022: Judge clicks and gaps within the segment, and defer pokes instead of erasing them

Date: 2026-10-25

Decision 0021's candidate still lost typing pokes, dropout pokes and many varied pokes:

- **Clicks held whole windows.** A click held every hop whose 43 ms window contained it, about
  ten hops, so pokes fell below the 80% valid-hop rule (71% in a traced poke). Holding fewer hops
  instead left clicks in the change map: the whole-window gain and timing fit absorbed a click
  near the window edge, and a 1–2% gain error is larger than a hand's echo.
- **The residual included the whole spectrum.** Only 8–20 kHz is predicted, so the microphone's
  own sound outside the band counted as residual and hid a click's rise, worst in loud passages.
- **Gaps erased trajectories.** A 10 ms gap invalidated ten hops and cleared the detector; the
  spec only forbids confirming a poke within 300 ms of a gap and allows 20% invalid hops.
- **A slowly moving hand raised the floor.** The floor learned from every hop below the on
  threshold, so a drifting hand's weak echo (z 3–7) raised it where the next poke would pass.

Opt-in parameters, all in residual mode with `change_window`:

- `change_in_band`: form the residual in the analysis band only.
- `change_own_fit`: fit the field's gain, phase and sub-sample timing over the segment itself.
- `change_blank_ratio` (4): zero residual samples whose envelope, relative to the predicted
  sound's, exceeds four times the segment's median, on every microphone, from 0.5 ms before
  the loud part to 4 ms after it. A click then leaves the hop valid.
- `change_impulse_ratio` (12): hold motion only when the segment's residual relative to its
  prediction exceeds twelve times its two-second median. Impulse hops are still measured but
  learn nothing, and reuse the last clean gains and fit.
- `change_gap_ms` (12): measure a window whose missing frames form no gap longer than 12 ms and
  cover at most a quarter of it, blanking samples that lost microphone audio or needed lost
  reference. Nothing is learned from it; the segment needs at least half its samples.
- `poke_defer_suspension`: within 300 ms of a gap or model jump, keep the trajectory and count
  unusable hops as invalid; a poke that meets every rule meanwhile is reported when the
  deferral ends. Pauses and source or preset changes still clear it.
- `floor_learn_fraction` (0.5): the floor learns only below half the on threshold.

For every configuration, a farther microphone's excluded direct region now moves only when its
direct sound moves by more than two taps; a flickering edge had let taps learn the region's
absence and then spike.

Across the 54 probe cases, 11 fail against 17. PC, stereo, compressed, both clocks and every
negative scene pass on every seed; dropouts and PC Bluetooth pass two of three, typing with
pokes two of three (8, 10 and 10 of 10), varied one (27, 20 and 23 of 30). Volume steps stay at 6
of 10: after each step the simulated speaker's soft clipping changes the scene for about a
second. Three failures remain basic latency (decision 0021). The full numbers are in
[tuning](../tuning.md). Defaults and gates do not change here.

Rejected: a floor that falls faster than it rises (varied improved but clocks, walking, PC and
typing failed: 20–26 cases); letting a persistently high tap's floor follow it (the high period
after a volume step is shorter than a safe release time); re-settling on a field-level jump (the
floor then calibrates on the unconverged background, and volume recall fell to 2–3 of 10);
adapting the background during long holds (it learned clicks: up to 61 false typing pokes); a
wider exclusion after a farther microphone's direct sound; and a longer presence hold-over.
