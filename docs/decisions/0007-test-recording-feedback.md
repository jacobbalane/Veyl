# 0007: Test recording feedback

Date: 2026-10-22

The plan defines consent, durations, Space marks and retention, but leaves recording failures,
listing and deletion feedback unspecified. M3.4 needs these controls before device recordings.

Use the plan's consent dialog and a visible recording banner. Show saved tests in Help, require a
named confirmation before deletion, and report partial or failed writes plainly. Stopping capture,
changing its configuration or disconnecting ends the current recording. Queue overflow ends it as
failed; capture and the engine continue. Delayed callbacks cannot cross consent periods.

Keep the shared folder format. Add recording status under an extra `recording` field in
`meta.json`; older readers ignore it. Missing frames are zero-filled in WAV files, while
`blocks.jsonl` retains only received blocks. Interrupted files are labelled incomplete on restart.
Retention and deletion touch only Veyl's dated directories and reject links or path traversal.

The additional words are recorded in Appendix C.8 and `docs/ui-copy.md`. This adds no upload or
automatic recording. M3 analysis reports stream statistics; echo and gesture results follow in M4.
