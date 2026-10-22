# Veyl Link

Coming in M1. Protocol v1 pairs an Android phone with a Windows PC by comparing a six-digit code.
The phone is the server, the PC connects out, and each session derives fresh encryption keys.
Audio is filtered before it reaches the link. Rust and Kotlin implementations will share exact
test vectors. The complete wire specification is section 4 of the workspace's `VEYL_PLAN.md`.

No foundation codec should be treated as a complete or secure transport until M1 is verified.
