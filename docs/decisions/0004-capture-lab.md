# 0004 — Debug capture Lab and phone listening wording

Date: 2026-10-22. Status: implementation of M2.1's explicit debug recording exception.

## Context

M2.1 requires a phone-only capture spike and optional ten-second WAVs so the maintainer can check
whether playback capture and the microphone work together. Production privacy rules prohibit
phone recordings. Appendix C does not yet name the Lab or its diagnostic fields.

## Decision

Put the Lab UI, WAV writer and recording controls in `src/debug`; release contains no recording
implementation. Open the Lab by holding the version label in a debug build. Audio stays in memory
unless the maintainer explicitly confirms a ten-second recording. Save at most ten seconds per
stream in app-private storage, with an explicit warning that these diagnostic WAVs are unfiltered.
Provide deletion and local ADB retrieval instructions. Never send, share, log or upload recordings.
Normal capture always passes both streams through the privacy filter before the null/link sink.

Add the exact diagnostic and interim listening strings in Appendix C.5 and `docs/ui-copy.md`.
Production strings continue to use C.3. M2's unpaired tests use a null link; the Lab explains that
there is no PC connection. G1 and later device results remain pending until the maintainer reports
them. Build the debug APK in CI even when release signing is available.

The explicit debug Lab also bypasses the ten-minute no-PC stop so the M2.7 thirty-minute
null-link device check can finish. It does not pretend a PC is connected. Production listening
still stops after ten minutes without a PC; locking the screen and all other stop rules still apply.
The Lab skips the automatic first-run sound check to exercise the recorder fallback directly;
Settings can run it explicitly. Production listening always checks when no result is saved for
the current Android fingerprint. Projection loss uses the lock alert only when the phone is
actually locked or its screen is off; other projection loss uses the capture-failure alert.

## Consequences

The requested hardware check is possible without adding phone recording to the shipped app.
Debug and release remain installable side by side. A successful build does not prove capture or
picture-in-picture on a real phone. No protocol or privacy-filter settings change.
