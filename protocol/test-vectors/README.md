# Veyl Link v1 vectors

`v1.json` contains public, deterministic test inputs, including private keys. Never use them for
live connections. X25519's pairing keys come from RFC 7748 section 6.1; session keys are different.
JSON bodies are strings because transcripts hash the exact UTF-8 bytes, including field order.
The AUDIO `plain` field includes its type byte; sealed `payload` fields do not.

Regenerate with `cargo run -q -p veyl-link --example gen_vectors > protocol/test-vectors/v1.json`.
Use a UTF-8 shell without a BOM. Rust regenerates and compares the bytes in its tests; CI also
compares the command's output. Kotlin loads this same file as a test resource and reproduces the
crypto, JSON transcripts, sealed frames, AUDIO, PING and PONG.
