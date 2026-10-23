# 0010: PC sound-check measurements before the live engine

Date: 2026-10-23

The PC sound check precedes M4's live engine in milestone order. It needs real measurements now,
without reporting a gesture score or saving diagnostic audio automatically.

On the user's click, play five seconds of BrightLoop through the selected render endpoint, at the
current Windows volume. Use a normal shared WASAPI float stream with Windows format conversion.
Keep COM, buffers and scheduling on its owner thread. Cancel on source/device changes or Quit;
Settings also offers Cancel. Never play automatically when selecting PC mode.

Tap only the existing privacy-filtered source blocks, through a bounded queue, and keep at most
seven seconds per channel in RAM. Collect for six seconds after playback starts to allow the
acoustic tail. Gaps, configuration changes, overflow and playback failure interrupt the check.
They are not presented as poor acoustics. Clear the tap on completion or cancellation.

A portable diagnostic aligns the high band by cross-correlation, then uses the phone check's
2048-point Hann/50% Welch coherence. Evaluate channel pairs and keep the best coherence×coverage
score. Report coherence, active bandwidth and lag. Initially, “hears its own sound well” requires
mean coherence at least 0.2 (the phone check's floor) and active bandwidth at least 2 kHz. These
diagnostic thresholds are provisional until M5's device tuning. They do not imply poke recall.
M4 will supply the live echo engine and its full bandwidth masks.

Additional playback notice, interruption and diagnostic wording is in Appendix C.9 and
`docs/ui-copy.md`. The result and UI fixtures contain numbers and device metadata only.
