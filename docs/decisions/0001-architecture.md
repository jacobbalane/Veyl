# 0001 — Architecture and product boundaries

Date: 2026-10-21. Status: accepted by the maintainer in `VEYL_PLAN.md`.

## Context

Veyl should turn the sound already playing into a way to control a Windows PC without a camera
or additional sensing hardware. Phones and PCs vary, so capabilities must be measured at run time.

## Decision

Ship phone mode and PC mode in v0.1. Android 8+ uses native Kotlin and Compose, with playback
capture on Android 10+ and the Veyl player and generated test sound as fallbacks. Windows 10 1809+
and Windows 11 on x64 and ARM64 use Tauri 2, Rust and React. A platform-independent Rust engine
serves both sound sources, the simulator and lab tool.

Use the playing sound as the reference. Keep the live quality meter honest and actions off by
default. In phone mode the phone is the server; the PC connects out and discovers phones through
Windows DNS-SD. Filter audio at its source, defaulting to an eighth-order 8 kHz high-pass, before
encryption or engine processing. Keep data local; allow audio storage only for an explicit test
recording on the PC.

Use an MIT-licensed monorepo, Glim's conventions, GitHub Actions and GitHub Pages. Follow the
milestone order. The maintainer owns device checks, account setup, signing secrets and publishing.

## Consequences

Every capability needs a fallback. Neither app may assume a phone model or specific PC hardware.
Both protocol implementations must share vectors. Simulation is useful verification, but does not
replace recordings and device acceptance. Later pointer control, mixed modes and boost sound
remain outside v0.1. This record summarizes the existing decisions without changing the design.
