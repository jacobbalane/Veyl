# 0009: Source choice and PC feedback

Date: 2026-10-23

The PC capture owners need a durable user choice before opening a microphone. Full onboarding is
scheduled for M7, after the echo engine and actions exist. Device setup and useful failure feedback
are needed now.

Show the plan's source-choice cards first, with the hardware recommendation. Save a separate
`source_choice_done` flag so this choice does not skip M7's placement and practice steps. Existing
settings without that flag still load. Phone remains the initial mode; selecting PC is deliberate.
Entering PC mode uses the Far default when the range still has its original Normal value.

One source owner closes phone networking before preparing PC capture and closes PC capture before
resuming phone discovery. A shared generation fences cancelled preparation and old callbacks. Device
notifications follow Windows defaults, while unavailable explicit choices stay selected. An invalidated
stream retries after 100 ms; other initialization failures retry after five seconds. Startup remains
cancellable. Real restart timing and capture performance remain device checks.

Diagnostics report actual formats, raw mode, resampling, buffer latency and scheduling. Drift stays
unavailable until the engine implements its estimator. Additional startup, missing-device, placement,
failure and diagnostic wording is recorded in Appendix C.9 and `docs/ui-copy.md`.
