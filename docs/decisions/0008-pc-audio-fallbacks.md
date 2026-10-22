# 0008: PC audio compatibility fallbacks

Date: 2026-10-22

Windows exposes optional endpoint properties and can reject a raw or event-driven configuration
even when the advertised capability suggests it should work. PC mode needs useful fallbacks while
preserving filtering, explicit source choice and honest diagnostics.

Try raw capture when advertised. If initialization rejects it, try ordinary shared capture. Try
event-driven capture first, with polling every five milliseconds as the fallback. Both streams
use AudioCategory_Other, never the communications category. Report the configuration that actually
initialized. Try MMCSS Pro Audio on each owner thread; if registration fails, keep capture usable
and report the missing scheduling hint in diagnostics. This cannot establish performance or
device acceptance, which remain subject to P1/P4/P5.

Keep each COM apartment, buffer, event and scheduling handle on its capture thread. Neither stream
starts until both initialize and the filtered source has a configuration. On failure, cancel both.
No raw buffer crosses that boundary or enters a recording. Missing endpoint names use the generic
“Audio device”; this additional wording is in Appendix C.9 and `docs/ui-copy.md`.
