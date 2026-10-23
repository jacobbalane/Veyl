# Working on Veyl

Veyl turns echoes of the sound already playing into gestures for a Windows PC. It supports an
Android phone or the PC's own speakers and microphone, with no camera or cloud service.

Read the workspace's `../VEYL_PLAN.md`, section 0, then `docs/progress.md`. Follow milestone order;
record completed work and verification there. Never mark missing or skipped checks as passed.

## Layout and commands

- `desktop/`: React UI and Tauri Windows host. `npm run -w desktop dev`, `typecheck`, `test`, `build`.
- `site/`: GitHub Pages home, docs and changelog. `npm run -w site dev` and `build`.
- `crates/`: DSP, Link, simulator and lab. `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p veyl-link -p veyl-desktop`.
- `android/`: JVM core and Android app. From that directory,
  `./gradlew :core:test :app:testDebugUnitTest :app:lintDebug :app:assembleDebug`.
- `protocol/`: shared test vectors. `assets/brand/`: editable brand and generated icons.
- `docs/`: user docs, decisions and progress. `lab/recordings/`: ignored recordings.
- Root checks: `npm ci`, `npm run format:check`, `node --test "scripts/*.test.mjs"`,
  `node scripts/check-version.mjs`. See `docs/development.md` for prerequisites and status.

Every DSP change runs `cargo test --release -p veyl-dsp -p veyl-sim -p veyl-lab`; scenario gates
arrive in M4. Every protocol change regenerates `protocol/test-vectors/v1.json` once M1.3 creates
it, and updates Rust, Kotlin and `docs/protocol.md` together.

UI text comes from `docs/ui-copy.md`: Android uses `strings.xml`, desktop uses `src/copy.ts`.
Use plain words in docs and comments. Record design changes in `docs/decisions/NNNN-title.md`
and append one line to section 15 of the plan. Use Conventional Commits and never force-push main.
Never commit secrets, signing keys, recordings or build output. Never create tags or releases.
Ask before installing system-wide tools. Account setup, secrets, publication and device tests
belong to the maintainer; prepare exact steps and keep their acceptance checks pending.

## Parallel work and local status

Use agents for independent tasks in the remaining milestones. Give each agent a clear scope and
file ownership or an isolated worktree. Review their changes and verify the combined result
before keeping it. Stop finished agents and their helper processes, preserving useful reports
and the maintainer's files and processes.

The maintainer's local status page lives under ignored `.cache/status/`. Never stage, commit,
publish or copy any of its files into the app or website. Keep its local server running when
cleaning up other helpers unless the maintainer asks to stop it.

## Privacy rules (hard requirements)

1. Audio leaves the phone only to a paired PC, only over the encrypted link, and only after the
   privacy filter. In PC mode, audio never leaves the PC, and the privacy filter runs right after
   capture.
2. Neither app writes audio to disk, except a test recording the user starts on the PC. Test
   recordings stay in the PC's app data folder, are deleted after 7 days by default, and are never
   uploaded.
3. No analytics, telemetry, crash reporting, accounts, ads or cloud services of any kind.
4. The only network traffic: Veyl Link on the local network, DNS-SD announcements, and GitHub only
   when the user opens a link or (from v0.2) checks for updates.
5. Logs never contain audio samples, keys, nonces or pairing codes.
6. The phone always shows that it is listening (the listening screen, a notification and Android's
   own indicator) and which PC is connected.
7. Lowering the privacy filter below 8 kHz requires a deliberate choice on the phone, with a
   warning.
