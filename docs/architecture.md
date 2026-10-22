# Architecture

Coming in v0.1. A native Android app or the PC's own speakers and microphone provides sound to a
shared Rust echo engine. React displays its meter and echo view inside a Tauri window.

Phone mode captures a digital reference and microphone input, filters both, then sends them over
an encrypted local connection to a paired PC. The phone listens for connections; the PC connects
out. PC mode filters Windows loopback and microphone input locally. Both feed the same engine.

The engine compares streams, learns the static room, tracks movement and recognizes a poke.
Actions start off. Simulation and lab analysis use the same engine without platform APIs.

Detailed thread and data-flow diagrams will be added with those implementations. See
[the architecture decision](decisions/0001-architecture.md) and [progress](progress.md).
