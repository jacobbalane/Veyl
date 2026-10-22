# Implementation progress

Last updated: 2026-10-22.

Section 0 followed: the full plan and the listed Glim references were read before writing source.
Work remains in milestone order. M0's implementation, GitHub setup and preview publication are
complete. The maintainer reports that both installed apps open: Windows shows the phone-search
placeholder and Android shows the waiting-for-PC placeholder. M0's listed acceptance criteria pass;
the additional device checks below remain unreported. M1 is complete. M2 is in progress; connecting the
apps is M3. The published 0.0.1 preview still contains the M0 interface skeleton.

The maintainer approved installing build tools and supplied the Git author identity. Rust,
Visual Studio C++ Build Tools, Java 17 and the Android command-line SDK are now installed. Git
identity is configured for this repository only. The earlier npm/network restrictions are resolved.

## M0 — Foundation

**Goal:** a complete, empty product skeleton. Both apps install and open, CI builds everything, the
website is live, and the release pipeline is proven end to end.

- [x] **M0.1** Create `projects/Veyl/` (`git init -b main`). Root files: `LICENSE` ("Copyright (c)
      2026 Veyl contributors"), `README.md` (outline §9.3; mark unfinished parts "Coming in v0.1"),
      `CONTRIBUTING.md`, `SECURITY.md`, `CHANGELOG.md` (with `## Unreleased`), `THIRD_PARTY_NOTICES.md`,
      `AGENTS.md`, `.gitignore`, `.gitattributes`, `.editorconfig`, `.prettierrc.json`,
      `.prettierignore`. `docs/` with `progress.md`, `ui-copy.md` (Appendix C), `decisions/0001-
architecture.md`, and stub files for the rest of §9.9.
- [x] **M0.2** Root `package.json` (workspaces `desktop` and `site`; scripts `format`,
      `format:check`, `version:bump`, `version:check`; dev dependency `prettier`). Cargo workspace with
      `[workspace.package] version = "0.0.1"` and the release profile. `rust-toolchain.toml`.
      `scripts/bump-version.mjs`, `scripts/check-version.mjs` and `scripts/changelog-section.mjs` with
      `node:test` tests in `scripts/*.test.mjs` (run by `node --test "scripts/*.test.mjs"` in CI).
- [x] **M0.3** Brand: `assets/brand/veyl-mark.svg`, `veyl-wordmark.svg`, `scripts/render-icons.mjs`
      → desktop icons (`npx tauri icon`), Android adaptive icon (vector foreground, `ink` background,
      monochrome layer), site favicons and `og.png`.
- [x] **M0.4** Crates: `veyl-dsp` with the real `filters.rs` (§5.5.5, with tests) and an `Engine`
      stub, plus `testsound.rs` with `BrightLoop` (§6.12.2) and its tests; `veyl-link` with frame
      constants and a codec test; `veyl-sim` and the `veyl-lab` CLI with `scenarios`.
- [x] **M0.5** Desktop skeleton: React shell (header, tabs, empty states, preview badge), Tauri app
      (window shown on `ui_ready`, tray "Open Veyl"/"Quit Veyl", single instance, close to tray,
      logging, `settings.json` with `pc_id`), one Vitest test, one Rust test.
- [x] **M0.6** Android skeleton: Gradle wrapper, version catalog, `:core` (`Base64Url` + tests),
      `:app` (theme, Manrope, Welcome screen, Home placeholder showing the version, strings from
      `ui-copy.md`, adaptive icon), release signing from environment variables, lint baseline.
- [x] **M0.7** Website skeleton: three pages, `chrome.tsx` with the download menu (Windows x64,
      Windows ARM64, Android), Markdown renderer, changelog page, first version of the home content (§8.4) saying "First
      release coming soon" until v0.1.0.
- [x] **M0.8** CI: `build.yml` (Windows x64 and ARM64; the vectors step stays commented out until M1.3), `site.yml`,
      `dependabot.yml`, issue and pull request templates.
- [x] **M0.9 [maintainer]** §10.7 steps 1–5: GitHub CLI, create and push the repo, turn on Pages,
      Android signing key and secrets, repository settings (homepage, topics, private vulnerability
      reporting, security alerts, labels).
- [x] **M0.10 [maintainer] [device]** Pipeline dry run: add `## 0.0.1 (YYYY-MM-DD)` with "Preview
      build. Nothing works yet; this release tests downloads." to `CHANGELOG.md`,
      `node scripts/bump-version.mjs 0.0.1`, commit, tag `v0.0.1`, push. The draft release must contain both installers (x64 and ARM64), the APK,
      their checksums and attestations. Publish it as "0.0.1 — preview build (no features yet)". Download both from the
      website; both install and open.

**Acceptance:** CI green on `main`; the website is live at `https://jacobbalane.github.io/Veyl/`;
v0.0.1 downloads from the site; both apps open on the maintainer's devices.

**Verify:**

```powershell
npm ci; npm run format:check; node --test "scripts/*.test.mjs"; npm run -w desktop test; npm run -w site build
cargo fmt --all -- --check; cargo test --workspace --exclude veyl-desktop
cargo test -p veyl-desktop                      # Windows with the toolchain, or in CI
cd android; .\gradlew :core:test :app:assembleDebug   # with the toolchain, or in CI
```

## Finished task notes

- **M0.1:** Initialized `main`; added repository docs, agent rules, exact UI wording, privacy rules,
  architecture decision, housekeeping files and all section 9.9 document stubs. Verified formatting,
  required paths and ignored secrets/recordings/build outputs. Section 15 records the decisions.
- **M0.2:** Added npm/Cargo workspaces, committed-version sources and both lockfiles. Fourteen Node
  tests cover version bounds, mismatches, tag notes, Markdown extraction, formatting preservation
  and refresh failures. A real `bump-version.mjs 0.0.1` refreshes both lockfiles successfully.
- **M0.3:** Added the circle-and-arcs mark, outlined Manrope ExtraBold wordmark, PNGs, Windows ICO,
  Android adaptive/monochrome icons, site favicons and social image. `render-icons.mjs` generated
  them successfully; artwork was visually inspected. Fonts and licenses ship locally.
- **M0.4:** Added the real eighth-order privacy filter, deterministic BrightLoop, idle Engine stub,
  frame constants/codec and lab scenario catalogue. Seven portable Rust tests pass in debug and
  release; filter response and test-sound band energy pass at both 44.1 and 48 kHz. Clippy is clean.
  Simulation, pairing and authenticated transport are later milestones.
- **M0.5:** Added the React shell, empty states and disabled foundation controls. Native Tauri hosts
  the window, tray, single-instance handling, lifecycle logs and atomic settings with a persistent
  random PC identity. The UI test and native settings test pass. An isolated Windows launch
  verified the real window, IPC, settings and actions off. Both x64 and ARM64 NSIS installers build
  locally. The ARM64 app has not been run on ARM hardware.
- **M0.6:** Added the verified Gradle wrapper, version catalog, JVM Base64Url and two tests, Compose
  Welcome/Home screens, local Manrope, adaptive icons, backup exclusions, environment-based release
  signing and an empty lint baseline. Debug and unsigned release APKs build. Debug and release
  lint report zero errors and warnings. No Android phone installation is claimed.
- **M0.7:** Added home, docs and changelog pages, shared download controls for Windows x64/ARM64 and
  Android, safe Markdown rendering and first-release wording. Three site tests, TypeScript and
  production builds pass. Chrome checked all three routes, download selection, 360 px layout,
  reduced motion and desktop navigation without page errors. Sample illustrations are labelled.
- **M0.8:** Added the Linux checks, Windows x64/ARM64, Android, tag-only draft-release and Pages
  workflow definitions, plus Dependabot and issue/PR templates. `actionlint` validates the workflows.
  The protocol-vector step remains commented until M1.3. Remote builds, deployment and the
  tag-only release job now pass; their evidence is recorded below.

## Verification

This table records the M0 release checks. M1's protocol verification is recorded in its section below.

| Check                                                   | Result                                                    |
| ------------------------------------------------------- | --------------------------------------------------------- |
| `npm ci`                                                | Passed from the committed lockfile                        |
| `npm run format:check`                                  | Passed                                                    |
| `node --test "scripts/*.test.mjs"`                      | 14 passed, none failed or skipped                         |
| `node scripts/check-version.mjs`                        | All version sources agree at 0.0.1                        |
| `node scripts/bump-version.mjs 0.0.1`                   | Passed, including real lockfile refresh                   |
| Desktop TypeScript, Vitest and production build         | Passed; 1 UI test                                         |
| Site Vitest, TypeScript and three-page build            | Passed; 3 tests                                           |
| `cargo fmt --all -- --check`                            | Passed                                                    |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed                                                    |
| `cargo test --workspace --exclude veyl-desktop`         | 7 passed                                                  |
| Release portable Rust tests                             | 7 passed                                                  |
| `cargo test -p veyl-desktop`                            | 1 passed                                                  |
| `cargo run -p veyl-lab -- scenarios`                    | Lists the 21 planned scenarios                            |
| Native Windows app launch with isolated data            | Passed: window, IPC, identity and disabled actions        |
| Android `:core:test`                                    | 2 passed, none failed or skipped                          |
| Android debug and release lint                          | Zero errors and warnings; baseline remains empty          |
| Android debug and unsigned release build                | Passed                                                    |
| Windows x64 NSIS packaging                              | Passed                                                    |
| Windows ARM64 NSIS cross-build                          | Passed; runtime check needs an ARM64 PC                   |
| Browser checks at desktop and 360 px widths             | Passed; screenshots remain in ignored `.cache/qa/`        |
| Workflow syntax and expressions (`actionlint`)          | Passed; ShellCheck is unavailable locally                 |
| GitHub Build Veyl                                       | Passed: checks, Windows x64/ARM64, signed Android release |
| GitHub Pages workflow                                   | Passed; site deployed successfully                        |
| Live website browser checks                             | All three pages passed on desktop and mobile              |
| Downloaded CI artifacts                                 | Installer checksums and permanent APK signer verified     |
| Android release signing                                 | Passed locally and in GitHub CI                           |
| Release attestations                                    | All three verified against the workflow, tag and commit   |
| Published website downloads                             | x64, ARM64 and Android downloaded; three checksums match  |

The Android app has no separate JVM unit-test sources yet: `:app:testDebugUnitTest` reports
`NO-SOURCE`. Its foundation logic is covered by `:core:test`, and its Compose screens require the
maintainer's device check. Gradle's conditional configuration/vital-lint tasks can report SKIPPED;
full debug/release lint ran and passed. Gradle also reports an upstream deprecated
`Configuration.setVisible` call. It is not an Android lint warning or a failed build.

## Local artifacts

- Windows x64: `target/x86_64-pc-windows-msvc/release/bundle/nsis/Veyl_0.0.1_x64-setup.exe`.
- Windows ARM64: `target/aarch64-pc-windows-msvc/release/bundle/nsis/Veyl_0.0.1_arm64-setup.exe`.
- Android debug: `android/app/build/outputs/apk/debug/app-debug.apk` (debug application id).
- Android signed release: `android/app/build/outputs/apk/release/app-release.apk` (permanent signer
  verified locally; phone installation remains pending).
- Website: `site/dist/`.

Build output and local checksums stay ignored. The x64 installer is about 2.18 MiB, ARM64 about
1.94 MiB, and the signed release APK below 1 MiB. These local artifacts have no GitHub provenance
attestation yet.

## GitHub and signing setup

**M0.9 is complete.** GitHub CLI is installed and authenticated as `jacobbalane`.
[The public repository](https://github.com/jacobbalane/Veyl) has Pages from Actions, its homepage and
topics, private vulnerability reporting, Dependabot security alerts/fixes and the `device-report`
label configured. The live home, docs and changelog pages passed browser checks on desktop and
mobile. [Main CI](https://github.com/jacobbalane/Veyl/actions/runs/39708987846) passed its checks,
both Windows installers and the signed Android release; the Website workflow passed as well.

The permanent RSA 4096-bit PKCS12 key was created at the maintainer's request, outside the
repository at `%USERPROFILE%\.veyl-signing\veyl-release.p12`. Its random 256-bit password is
encrypted for this Windows account in `veyl-release.password.dpapi`; directory access is restricted
to the current user and SYSTEM. All four GitHub signing secrets work. Local and downloaded CI APKs
verify against the permanent certificate:
`a04d54b735f80adbf8c2549afb74883b019781e3340c57c5755ded7a0ac697c3` (SHA-256).

The maintainer confirmed that the keystore and actual password are saved in 1Password. Protected
local signing files remain the other safe copy required by plan section 10.7. Backup instructions
are in `%USERPROFILE%\.veyl-signing\BACKUP-INSTRUCTIONS.txt`; the DPAPI file alone is not a portable
password backup. The requested plaintext export, `../VEYL_SIGNING_SECRETS.txt`, is deleted and was
never committed. Do not recreate it without the maintainer asking.

CI uses the explicit `scripts/*.test.mjs` glob accepted by Node 24. Signed Android builds run the
available debug app unit-test task plus core tests, release lint and signed packaging; AGP does
not create a release app unit-test task here. Workflow action tags were checked against their
current upstream releases and pass `actionlint`. The latest MSVC environment action still declares
Node 20; GitHub runs it with Node 24, and Windows builds pass.

## Published preview

**M0.10 is complete.** The maintainer reports that the installed PC and phone apps both open.
The search/wait placeholders are expected: discovery and connection arrive in M3. With explicit
approval to create the tag and publish, `v0.0.1` points to
`f59be9e5ec5b8c0ef4a9e02e72cb8502f5697b00`. Its
[tag build](https://github.com/jacobbalane/Veyl/actions/runs/39711334196) passed all five jobs,
including attestations and the draft release. The changelog and release body contain the exact
preview notice. Version checks, lockfile refresh, fourteen Node tests and formatting passed.

[0.0.1 — preview build (no features yet)](https://github.com/jacobbalane/Veyl/releases/tag/v0.0.1)
was published on 2026-10-22. Before publication, all six files were downloaded from the draft:
both Windows installers, the signed APK and their three SHA-256 files. Checksums, installer/APK
versions, Android application ID and permanent signing certificate passed. Each binary's GitHub
attestation verified against this repository, `.github/workflows/build.yml`, `refs/tags/v0.0.1`
and the exact source commit, with self-hosted runners denied.

After publication, the unauthenticated latest-release API returned this release and all six
assets. Chrome clicked the live site's real download menu links for Windows x64, Windows ARM64
and Android. All three public downloads succeeded and matched both the draft's verified hashes
and their public checksum files. All three site pages passed at desktop and 360 px widths, with
no page errors, missing first-party assets or horizontal overflow. Evidence and downloaded files
remain ignored under `.cache/release/v0.0.1-tag/`, `.cache/release/v0.0.1-public/` and `.cache/qa/`.

## Waiting on device tests

The preview is published, its public downloads are verified, and the maintainer reports that both
installed apps open. Detailed tray/theme/orientation checks and device versions have not been
reported; keep those checks pending:

1. Download the appropriate Windows installer and Android APK from
   [the website](https://jacobbalane.github.io/Veyl/). Public download integrity and attestations
   were verified above; install the files on the actual devices.
2. On Windows, install and open Veyl. Confirm the title, tabs and version; actions stay unavailable.
   Close the window, reopen it from **Open Veyl** in the tray, launch a second instance, then use
   **Quit Veyl**. Report any failure.
3. On Android, install and open Veyl. Confirm Welcome, **Get started**, Home and version 0.0.1.
   Check light/dark themes, portrait/landscape and scrolling on the available phone.
4. Report Windows/Android versions, processor architecture, artifact versions and pass/fail for
   each step. The ARM64 installer needs an ARM64 PC for a real launch check.

No sensing or audio tests are appropriate for the 0.0.1 foundation. Those begin in later milestones.

## M1 — Veyl Link

**Goal:** the complete protocol (§4) in Rust and Kotlin, proven identical by shared test vectors.

- [x] **M1.1** `veyl-link`: frame codec (blocking and Tokio), all handshake and control messages
      (`serde`), AUDIO and PING/PONG codecs, crypto (`x25519-dalek`, `hkdf`, `hmac`, `sha2`, `aes-gcm`,
      `subtle`), `SealedChannel` with strict counters, pairing and session state machines written
      "sans I/O" (message in → messages out + new state) for both roles, and async helpers `pair()` and
      `session()` for the PC role.
- [x] **M1.2** Tests: round trips, size limits, unknown fields and types, a flipped bit closes the
      connection, skipped or repeated counters close it, wrong keys fail, timeouts with a fake clock,
      rate limiting, `proptest` for the codecs, an in-memory client ↔ server run of pairing and session.
- [x] **M1.3** `examples/gen_vectors.rs` and the committed `protocol/test-vectors/v1.json`; a test that
      regenerates and compares; enable the CI vectors step.
- [x] **M1.4** Kotlin `:core`: `Frames`, `Messages`, `Crypto`, `SealedChannel`, `PhonePairing`,
      `PhoneSession`, `AudioBlock`, `PingPong`, with JUnit tests that reproduce every vector byte for byte
      and test the phone-side state machines.
- [x] **M1.5** `docs/protocol.md` (§4 with Mermaid sequence diagrams).

**Acceptance:** all protocol tests pass in both languages in CI; the vectors match; docs complete.

**Verify:** `cargo test -p veyl-link`; `cargo run -p veyl-link --example gen_vectors` equals the
committed file; `.\gradlew :core:test`.

**M1.1–M1.3:** Rust now has bounded blocking/Tokio framing, all message codecs, X25519/HKDF/HMAC,
strict AES-GCM counters, pairing/session machines for both roles, and async PC helpers. The helpers
retain partial reads/writes across cancellation and service both directions while confirmations
are sent. Twenty-six Rust tests pass, including a full pairing/session run over a 19-byte duplex
buffer, fake-clock timeout, codec property tests, rate limits, unknown extensions, corruption,
replay, counter exhaustion, audio queue loss and liveness. Clippy passes with warnings denied.
The deterministic vector generator matches the committed file and RFC 7748's X25519 known answer;
CI's vector comparison is enabled.

**M1.4–M1.5:** Kotlin reproduces every committed vector byte for byte: X25519, commitment, raw JSON
transcripts, HKDF, the comparison code, both confirmation MACs, session keys and all six sealed
frames. Seventeen core JVM tests pass, including malformed messages, counter/authentication failures,
pairing/session gates, timeouts, key cleanup and the phone audio queue. Both languages prepare one
final `bad_message` response for authenticated malformed AUDIO, discard channel keys and prohibit
reuse. Debug APK/lint and a signed release build/lint pass; the permanent signer is unchanged.
Protocol documentation now includes the complete spec, message examples, two Mermaid sequence
diagrams and caller responsibilities. Android uses only Bouncy Castle's low-level X25519, with
its license bundled; other crypto uses the platform APIs. Kotlin sources are formatted with
ktfmt 0.65's four-space style. Both roles can cancel before a code exists; the PC helper services
that decision immediately and bounds the final abort write.

**M1 acceptance passed.** [Build Veyl](https://github.com/jacobbalane/Veyl/actions/runs/39730717387)
passed at source commit `865285f2f0ee7636ee9e292ab4d577437513f99b`: checks (including all 26 Rust
tests and the exact CLI vector comparison), Android (all 17 core JVM tests, release lint and signed
APK), Windows x64 (native clippy/tests and installer) and Windows ARM64 (cross-built installer).
The tag-only draft-release job is not applicable to this main push.
[Website](https://github.com/jacobbalane/Veyl/actions/runs/39730717483) also passed and deployed.
Local formatting, debug/release Android lint, both APK builds and permanent-signer verification
passed. App discovery, pairing UI, protected stores and streaming remain M3.

## M2 — Phone listening

**Goal:** the phone captures the digital copy and the microphone reliably, filters and packs them,
and handles every state before any PC is involved.

- [ ] **M2.1 [device]** Hidden debug Lab: consent → service → playback capture and microphone,
      live levels, sample rate, microphone source, channel counts, effects and optional 10-second
      app-private WAVs. **Gate G1:** YouTube Music runs with both streams for five minutes without
      errors; reference energy exists above 8 kHz; the microphone hears the music. Apply §14 R1 and
      record a decision if it fails.
- [ ] **M2.2** `ListeningService` and `CaptureRun` (§5.5): source selection, effects, timestamps,
      privacy filter, packer, routes and stop/error handling. A null link counts blocks until M3.
- [ ] **M2.3** `BrightLoop`, test-sound playback, Veyl player, FFT, sound check and runner; Android
      8–9 capability fallbacks. Test sound and the player require no projection consent.
- [ ] **M2.4** Listening screen, keep-screen-on, dimming, picture-in-picture, notifications and alerts.
- [ ] **M2.5** Welcome, permissions, Home without a PC, placement, Settings, About and Licenses.
- [ ] **M2.6** Tests: privacy filter, block statistics, BrightLoop, FFT, sound check, route
      classification and state reducers.
- [ ] **M2.7 [device]** D5, D6, D7, D15, D16 and D17 when Android 8–9 hardware is available;
      30-minute YouTube Music run; test sound/player without projection consent. Record results.

**Acceptance:** unit tests pass in CI; device results recorded; failed checks have their documented
fallbacks applied and a decision record.

**Verify:** `.\gradlew :core:test :app:testDebugUnitTest :app:lintDebug :app:assembleDebug`.

**M2.1 implementation ready; G1 pending.** Debug builds have a hidden Lab (hold the version label),
single-use projection consent, simultaneous playback/microphone readers, both pre-filter levels,
filtered high-band levels, counts, rate, source, channels, effects and metadata-only diagnostics.
Optional, explicitly confirmed raw WAVs are capped at ten seconds per stream in app-private
storage. Both the writer and Lab are debug source files. DEX inspection found the recorder in the
debug APK and no recorder class/private recording path in the release APK. The permanent release
signer is unchanged. CI now builds a separate `debug-android` artifact even when signing exists;
its name keeps it out of release assets. See [decision 0004](decisions/0004-capture-lab.md).

**M2.2 underway.** Capture tries 48/44.1 kHz with the specified microphone source/channel order,
disables available effects, retains active microphone metadata, packs exact ten-millisecond blocks
with extrapolated timestamps and raw statistics, then filters both streams before the null sink.
Route/silenced warnings and user, screen-lock, projection, recorder, permission, no-PC and thermal
stop paths are implemented. Locks renew while capture runs and cleanup closes recorders/effects.
Stored sound-check choices and changes to the own-player/test-sound sources follow in M2.3; keep
this checkbox open until those are integrated. Listening/PiP, dimming and notification groundwork
is present; settings and the rest of the M2 screens remain unfinished.

**Local verification:** 23 core JVM tests and three app JVM tests pass. New tests cover filter
response at both rates, stability/channel isolation/reset, pre-filter statistics and clipping,
timestamp extrapolation, the production ten-minute no-PC deadline versus the explicit debug
thirty-minute check, output routes and bounded little-endian WAVs. Debug/release builds and Android
lint pass with no warnings. Formatting passes. These checks do not verify hardware capture.

### Waiting on M2 device tests

No Android device is connected through ADB. Gate G1 and M2.7 remain pending. Follow the complete
[phone device-check instructions](phone-device-checks.md); for the first check:

1. Install `android/app/build/outputs/apk/debug/app-debug.apk` as **Veyl (debug)**, or use
   `debug-android` from the M2 CI run. The website's published 0.0.1 APK has no Lab.
2. Hold **Version 0.0.1**, play YouTube Music through the speaker, tap **Start capture check**, allow
   microphone access, read **One more step**, then approve Android's consent dialog.
3. Run for five minutes with Veyl visible or in picture-in-picture. Both block counts must keep
   increasing, Reference → High sounds must respond above silence when the track has high sounds,
   and the microphone must hear the music. Optional WAVs require an explicit recording confirmation.
4. Tap **Copy diagnostics**, then **Stop**. Report that metadata, the source commit/APK, phone and
   Android versions, pass/fail and any problem. Do not upload audio. Record failed checks and apply
   §14 R1 before acceptance.

Under §0.1.2, continue the remaining work that can be tested without hardware. The later thirty-minute,
route, lock, PiP, player and sound-check checks remain open; unavailable devices are not passes.

## Dependency follow-up

[Dependabot alert 1](https://github.com/jacobbalane/Veyl/security/dependabot/1) concerns the
Linux-only GTK dependency `glib` 0.18.5
([RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html)). Dependabot's update
attempt reports `security_update_not_possible`: the current GTK dependency chain cannot resolve
the fixed 0.20.0 release. The alert remains open. `cargo tree --locked -p veyl-desktop --target
x86_64-pc-windows-msvc --invert glib@0.18.5` and the ARM64 equivalent both report no dependency:
this crate is not compiled into either supported Windows binary. Android does not use GTK. This
separate Dependabot maintenance failure is not a Build Veyl test or packaging failure.
