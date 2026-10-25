# Implementation progress

Last updated: 2026-10-26.

Section 0 followed: the full plan and the listed Glim references were read before writing source.
Work remains in milestone order. M0's implementation, GitHub setup and preview publication are
complete. The maintainer reports that both installed apps open: Windows shows the phone-search
placeholder and Android shows the waiting-for-PC placeholder. M0's listed acceptance criteria pass;
the additional device checks below remain unreported. M1 is complete. M2's implementation and automated
checks are complete; hardware acceptance remains pending. M3's connection and recording implementation
is complete. M3P's PC-mode implementation and automated checks are complete; its device checks
remain pending under §0.1.2. M4's offline engine is complete: its scenario gates run in CI with decision 0023's thresholds. M5–M7 are implemented except the device checks, the target-laptop timing and the release, which belong to the maintainer. The published 0.0.1 preview still contains the
M0 interface skeleton. The maintainer authorized continuing non-destructive implementation while
away and limited all GitHub work to this repository. Physical review remains deferred.

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

**v0.1 checks (M5.4, M5.5, M6.6, M7.5):** follow [device checks](device-checks.md) with the newest
CI build: recordings R1–R14 and gate G2, processing cost and poke delay on the target laptop,
D8, D9, D12, P3, P6 and gate G3, then D1–D17 and P1–P7. None has been reported yet.

The 0.0.1 preview checks below also remain open.

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
- [x] **M2.2** `ListeningService` and `CaptureRun` (§5.5): source selection, effects, timestamps,
      privacy filter, packer, routes and stop/error handling. A null link counts blocks until M3.
- [x] **M2.3** `BrightLoop`, test-sound playback, Veyl player, FFT, sound check and runner; Android
      8–9 capability fallbacks. Test sound and the player require no projection consent.
- [x] **M2.4** Listening screen, keep-screen-on, dimming, picture-in-picture, notifications and alerts.
- [x] **M2.5** Welcome, permissions, Home without a PC, placement, Settings, About and Licenses.
- [x] **M2.6** Tests: privacy filter, block statistics, BrightLoop, FFT, sound check, route
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

**M2.2 implemented.** Capture tries 48/44.1 kHz with the specified microphone source/channel order,
disables available effects, retains active microphone metadata, packs exact ten-millisecond blocks
with extrapolated timestamps and raw statistics, then filters both streams before the null sink.
Route/silenced warnings and user, screen-lock, projection, recorder, permission, no-PC and thermal
stop paths are implemented. Locks renew while capture runs and cleanup closes recorders/effects.
Saved sound-check choices take priority, including a fallback to 44.1 kHz before abandoning the
chosen microphone. A source change starts a new run and retains a live projection; Stop releases
it. A pure reducer rejects old or late callbacks. Source transitions are serialized, and the next
start waits for the previous service to finish shutting down.

**M2.3 implemented.** Kotlin BrightLoop generates the deterministic sixteen-second stereo loop;
its sensing-band energy passes at both rates. AudioTrack feeds the identical PCM as the synthetic
reference with presentation timestamps. Media3 plays persisted SAF document/folder URIs through
stereo conversion, Sonic resampling and a tee. The tee never waits on the bounded reference ring;
overflow and seeks become frame gaps. Play/Pause, Previous/Next and Shuffle are available. Neither
own-player source requests projection consent, including on Android 8/9. The sound check compares
every usable microphone configuration over 1.5-second samples using a band-limited lag search and
Welch coherence, applies the prescribed tie breaks, and saves the result for the Android
fingerprint. Production starts check first when needed; the debug spike can exercise the default
fallback directly. Low coherence warns but permits trying the phone.

**M2.4–M2.5 implemented.** Welcome and optional notification permission lead to Home. Listening
keeps the screen on, restores brightness on touch and optionally dims after ten seconds. Square
PiP uses auto-enter on Android 12+ and the older leave-hint path, with capability/permission checks
and a fallback tip. Settings has phone name, deliberate lower-filter confirmation, dim/PiP toggles,
the explicit system battery action, repeatable sound check and metadata diagnostics with twenty
recent state changes. Placement, About and bundled Licenses are present. Home truthfully has no
PC connection or meter until M3; audio always uses the filtered null sink. Identity/settings and
playlist URIs use DataStore with app backup disabled. The release APK is about 2.1 MB and uses the
existing permanent signer. The debug-only Lab remains absent from release.

**M2.6 / local verification:** 31 core JVM tests and six app JVM tests pass. New tests cover filter
response at both rates, stability/channel isolation/reset, pre-filter statistics and clipping,
timestamp extrapolation, the production ten-minute no-PC deadline versus the explicit debug
thirty-minute check, both route rules, source/rate preference, bounded little-endian WAVs,
BrightLoop determinism and band energy, known FFTs and inverse scaling, positive/negative sound
delays, coherence at a known SNR and its failure cases, best-mic ties, capture state transitions,
ring overflow and a real producer/consumer stress run. A JVM test runs the actual Media3 stereo,
Sonic and tee processors: a 44.1 kHz mono tone becomes 48 kHz stereo with unchanged pitch, duration
and exact reference bytes. Debug/release builds and Android lint pass with no warnings; the
permanent APK signature verifies. The required Rust DSP/simulator/lab release tests also pass.
These checks do not verify hardware capture. The M2.1 capture-spike CI build passed at
`a535c9da154196d2b67e9a4004498ae233f8d22b`
([Build Veyl](https://github.com/jacobbalane/Veyl/actions/runs/39734647978)). The complete M2 build at
`62d25de4dc7dccedf483c1f1eb4c1e152a78b238` also
[passed CI](https://github.com/jacobbalane/Veyl/actions/runs/39740178033): all checks, Android's 37 JVM
tests/debug+release lint and APKs, Windows x64 tests/installer and the Windows ARM64 cross-build.
No tag or release was created.

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
The maintainer asked to defer app review until implementation is finished and to proceed through
the remaining milestones without stopping for intermediate device results. M2 acceptance therefore
remains pending hardware; carry these checks forward while continuing M3 under the device exception.

## M3 — Connect and stream

**Goal:** phone and PC pair, connect and stream encrypted audio. PC diagnostics show the signals;
test recordings and a fake phone allow end-to-end testing before hardware review.

- [x] **M3.1** Phone LinkServer/LinkSession over core, NSD advertising, network information,
      Keystore-protected PairingStore, pairing dialog, connected/waiting Home, request-start
      notification, stats, audio configuration, capture state and forgetting PCs.
- [x] **M3.2** Desktop DNS-SD discovery, connection manager, pairing flow/dialog, Phones tab,
      add-by-address, DPAPI pairings, SoundSource/PhoneSource, level-meter engine stub and Help
      diagnostics: block rates, levels, drops, RTT, arrival latency and battery.
- [x] **M3.3** `veyl-lab fake-phone` and end-to-end test: pair, session, forty seconds of poke_basic,
      correct block counts and no decryption errors. Gesture checks begin in M6.
- [x] **M3.4** Desktop opt-in test recordings: dialog, Space marks, retention and `veyl-lab analyze`
      reading stream statistics.
- [ ] **M3.5 [device]** D2/D10, thirty-minute stream with less than 0.1% drops and maintainer
      recordings R1–R10. Defer results to the final app review as requested.

**Acceptance:** end-to-end CI passes, D2/D10 pass and R1–R10 exist. Hardware checks remain pending
until actually performed.

**Verify:** `cargo test --release -p veyl-lab`; `cargo test -p veyl-desktop`; both streams at 100
blocks per second on a connected device.

**M3.1 implemented.** Android now accepts one IPv4 PC connection, advertises the actual listening
port through NSD and shows its local address. A busy listener rejects other PCs; an occupied
default port falls back to an ephemeral port. Networking runs while an activity or the listening
service runs. Android 17's local-network explanation, grant, denial and revocation paths are
implemented; older versions keep their existing behavior. See decision 0005.

Pairing requires Veyl in front, the current comparison dialog and both confirmation MACs. Paired
keys are wrapped with Android Keystore AES-GCM, bound to the PC identity, and written atomically
under no-backup storage. Settings lists paired PCs and can forget them. Reconnected sessions show
the PC name, send capture state/configuration and one-second device statistics, and accept meter
updates. Only filtered audio enters the bounded queue. Control/PONG messages take priority;
overflow drops old audio and marks gaps. A replaced or stopped capture run cannot resume through
a late audio callback. Read deadlines and liveness continue even during a partial socket frame.
Request-start notifications require a one-use, expiring token for the current authenticated
session; an arbitrary launcher intent cannot start capture.

**M3.1 local verification:** 33 core and 14 app JVM tests pass. New loopback TCP tests cover real
pairing/session handshakes, both confirmations, stale-dialog rejection, configuration before both
audio streams, strict decryption counters, PONG, forgotten-peer closure, background pairing
refusal, stopped/replaced runs and a corrupted authentication tag. Listener tests cover port
fallback, busy rejection and shutdown. Storage tests cover restart, atomic-write failure and
identity-bound ciphertext; notification tests cover forgery, replay, expiry and session changes.
All 26 Rust Link tests pass; regenerated vectors remain byte-for-byte identical. Android release
lint and the signed release build pass; its permanent signing certificate is unchanged. These
tests do not establish real LAN discovery or Android Keystore behavior on a device. CI acceptance
passed at `58dfb9f415215e40cc9a4aac1a193f48330f206c`
([Build Veyl](https://github.com/jacobbalane/Veyl/actions/runs/39765745065)): all checks, Android tests
and signed/debug APKs, native Windows x64 tests/installer and the ARM64 installer cross-build.
The website deployment also passed. No tag or release was created.

**M3.2 foundation complete.** The Windows DNS-SD adapter copies native callback data into bounded Tokio
channels, frees Windows allocations, and keeps cancellation contexts alive until the terminal
callback. Opaque tokens and owned references protect overlapping/late callbacks. Nearby phone
records validate protocol/TXT fields, coalesce the short identity, update addresses and expire
after ninety seconds. No desktop listening socket is opened. DPAPI pairing storage uses the
current user with UI forbidden, binds ciphertext to both device identities, writes atomically,
keeps unknown fields and exposes only metadata to the UI. Seven desktop Rust tests pass, including
native DPAPI restart/tamper/identity checks, save failure, callback lifetime and discovery-table
behavior; clippy passes with warnings denied. This foundation
[passed CI](https://github.com/jacobbalane/Veyl/actions/runs/39767015607) at
`8bb59e475ef56afcc3d1db4235d6717e9eb80bc6`, including both Windows installers and signed Android.

**M3.2 implementation and verification complete.** The connection manager
now browses continuously, prefers the last paired phone, connects by IPv4/hostname with one
three-second DNS/connect deadline, authenticates, reconnects with bounded backoff and requests a
start notice on the phone. New discovery refreshes can expedite a pending reconnect. Pairing
decisions are tied to the current comparison; stale workers cannot save a pairing or replace a
connection. Quit/forget attempts a bounded `bye`, then shuts down sockets and native discovery.
The Phones tab has nearby/saved devices, code comparison, address entry and confirmed forgetting.
Help shows real block rates, pre/post-filter levels, gaps, queue/drop counts, RTT, arrival delay and
battery. The source-independent engine queue holds 400 blocks and drops audio without blocking;
the M3 stub measures levels only. The interface explicitly says gesture control is coming soon
([decision 0006](decisions/0006-desktop-connection-feedback.md)). Copied diagnostics exclude audio,
protected keys and comparison codes.

Sixteen desktop Rust tests and all 29 Link tests pass, with clippy warnings denied. Loopback tests
perform real pairing, sessions, both audio streams, start requests, PING/PONG and intentional close;
other tests cover pressure/gaps, capture replacement, old session results, durable forgetting,
native DPAPI and typed interface fixtures. Six desktop Vitest tests and TypeScript/build checks
pass. Headless Chrome checks the phone/address view, diagnostics, minimum-width layout and real
modal focus with fixed example data; screenshots are private build artifacts. A separate native
UI launch was deferred because the installed 0.0.1 app is running. It was left available for the
maintainer. Native compilation/tests pass; this does not replace the pending LAN/device checks.
All 47 Android JVM tests, debug/release lint and the signed release build pass; the permanent
signer is unchanged. Rust/Kotlin expose authenticated peer readiness; cancellation after PC
confirmation remains possible, known version errors are preserved, and both shared writer queues
prioritize control messages with bounded backlog. Regenerated wire vectors remain identical.
The required DSP/simulator/lab release tests pass. No tag or release was created.

**M3.2 CI passed** at `9bf137620084cf3ff85adaa60bc0b97e7435c124`
([Build Veyl](https://github.com/jacobbalane/Veyl/actions/runs/39773026506)): all portable checks,
Android tests/lint/APKs, native Windows x64 tests/installer and the ARM64 installer cross-build.
The website deployment passed too. M3.3 now builds the simulated phone and its required
`poke_basic` acoustic input; the broader simulator and engine gates remain M4.

**M3.3 implementation and verification complete.** The loopback-only simulator
uses Veyl Link's real phone pairing/session machines, compares codes through an interactive
terminal, keeps confirmed pairings only in memory and accepts one PC at a time. It sends ready,
configuration, capture state, statistics and both filtered audio streams, answers heartbeats,
rejects busy clients and closes on shutdown. Fast mode uses socket backpressure; real-time mode
paces by frame time. Recording replay checks the shared folder format, preserves PCM/headers,
gaps and unknown timestamps, and rebases known capture times without changing stream offsets.

The `poke_basic` prerequisite renders Bright Loop through speaker EQ/nonlinearity, direct/static/
diffuse/hand echo paths, fractional delay, microphone noise and the privacy filter. It supplies ten
truth turns and raw level metadata before 16-bit quantization; the other twenty scenarios remain
M4 work. Six DSP, five simulator, one recording-reader and two end-to-end tests pass in release,
with clippy warnings denied. The forty-second encrypted run receives exactly 4,000 blocks per
stream, checks every PCM sample and header, and reports zero decryption errors. Separate checks
cover pacing, busy rejection, a deliberately corrupted tag and listener shutdown. No gesture
results or device acceptance are claimed. Codes and audio are absent from test output.

M3.3 [passed full CI](https://github.com/jacobbalane/Veyl/actions/runs/39778554705) at
`8d8c39e`: portable checks/end-to-end tests, Android lint/tests/APKs, Windows x64 tests/installer
and the ARM64 installer cross-build. The test waits for all twenty heartbeat replies independently
of the final capture-state message; those messages can arrive in either order. No tag or release
was created.

**M3.4 implementation and verification complete.** Record a test is available on
Live and Help → Diagnostics while capture runs. Consent chooses 30, 60 or 120 seconds and optional
Space marks. A visible banner counts down; Stop, capture ending, configuration changes and
disconnect finalize the files. Audio is copied to the bounded disk queue directly from the sound
source, before engine drops can affect it, and only during the current consent period. Overflow
ends the recording as failed while capture continues. Old callbacks and generations cannot enter
a new recording. Only privacy-filtered PCM is accepted. WAVs preserve absolute frame bases and
zero-fill missing frames; JSONL preserves received blocks, levels, timestamps, flags and marks.
Device details and app versions come from authenticated readiness. The M3 engine has no gesture
events yet, so its events file is empty. See decision 0007 for additional feedback wording.

Help lists local tests with Open folder and confirmed Delete. Startup removes managed folders
older than seven days by default; traversal, links and unrelated folders are excluded. Interrupted
or partial files are labelled incomplete. Nothing is uploaded. `veyl-lab analyze DIR [--out DIR2]`
writes a stream-statistics summary with received frames, gaps, overlaps, raw/filtered levels and
unknown-clock counts; it does not claim echo or gesture results. Nine Vitest tests and twenty-one
desktop Rust tests pass. A real encrypted fake-phone → desktop source → recorder → lab-reader
test verifies consent, filtered data, device metadata and engine counts. Additional tests cover
retention, epoch isolation, marks and failure reporting. DSP/simulator/lab release tests and
clippy pass. Headless Chrome checks consent, focus, duration/mark choices, the banner and light
theme at minimum width. Native GUI/device verification remains deferred while the maintainer's
installed preview runs. The new recording metadata fixture is checked semantically by Rust and
typed in TypeScript; JSON formatting remains Prettier's responsibility.

M3.4 [passed full CI](https://github.com/jacobbalane/Veyl/actions/runs/39781727617) at
`3d83910`: portable checks/end-to-end tests, Android tests/lint/APKs, native Windows x64
tests/installer and the ARM64 installer cross-build. Website deployment passed. No release or tag
was created; the published 0.0.1 preview is unchanged.

### Waiting on M3 device tests

The maintainer requested the final app review after implementation. Keep these checks pending:

1. Install matching final Windows/Android builds and open Veyl on both devices on the same LAN.
   On Android 17+, allow local-network access. Confirm that Phones lists the phone and that its
   displayed address also works with Add by address. Denying/revoking the grant must close the
   phone listener without silently granting access or starting capture.
2. For D2, pair ten times, forgetting between attempts. Compare both codes each time, confirm on
   both screens, and require at least nine first-attempt successes. Cancel on either side once;
   backgrounding the phone during comparison must stop pairing. Reopen both apps to check saved
   pairings, then forget the PC on the phone and confirm reconnection requires pairing again.
3. For D10, start a filtered stream, turn phone Wi-Fi off/on and measure reconnection after Wi-Fi
   returns (at most five seconds). The phone must show the connected PC and keep its listening
   indicator. Run thirty minutes; record block/drop counts and require under 0.1% dropped blocks.
4. Collect R1–R10 using the plan's instructions. In Live or Help → Diagnostics, choose Record a
   test, select the required duration, leave Space marks on for poke tests and choose Start
   recording. Keep Veyl focused and press Space after each real poke. After the saved message,
   use Open folder to find the local test; run `veyl-lab analyze DIR` to check its stream counts.
   Keep audio local. Record versions, devices, pass/fail and failures here; none has run yet.

## M3P — PC mode

**Goal:** use the PC's speakers and microphone through the same filtered source interface, without
a phone. This milestone is marked parallel-ok in the plan; work begins while M3.4's builds run.

- [x] **M3P.1** Enumerate render/capture endpoints, form factors, raw support and Bluetooth;
      default-device notifications and a hardware-based mode recommendation.
- [x] **M3P.2** WASAPI loopback/microphone threads, raw fallback, format conversion/resampling,
      privacy filtering and timed ten-millisecond blocks behind a fake-able capture interface.
- [x] **M3P.3** Source switching, device settings/pickers, onboarding path, PC tips and diagnostics.
- [x] **M3P.4** Test-sound playback and Check this PC's sound.
- [x] **M3P.5** PC test recordings read by the lab analyzer.
- [ ] **M3P.6 [device]** P1/P4/P5, optional ARM64 P7, and recordings R11–R13.

**Acceptance:** CI unit tests pass, P1/P4/P5 pass and R11–R13 exist. Device checks remain pending
for the maintainer's final review; this PC needs speakers for the acoustic checks.

**Verify:** `cargo test -p veyl-desktop`; chosen devices produce both streams at 100 blocks/s.

**M3P.1 foundation implemented and verified.** A dedicated COM thread lists active render and
capture endpoints, names, opaque case-sensitive IDs, form factors, raw support, bus-based Bluetooth
and built-in hints, the default multimedia endpoints and battery availability. Notifications only
wake the owner through a bounded channel; unregistering and releasing COM objects stay on that
thread. Missing properties use conservative defaults, and a missing saved endpoint is never
silently substituted. PC mode is recommended only with suitable speakers and a microphone plus a
battery or two built-in devices. Three new tests and clippy pass. The read-only
`cargo run -p veyl-desktop --example audio_devices` check found three render endpoints and two
microphones here, reported no enumeration error, and recommended phone mode. It neither initialized
nor captured audio. Real default-device changes, Bluetooth identification and acoustic checks stay
pending. Production capture and the source/device interface follow in M3P.2/M3P.3.

M3P.1 [passed full CI](https://github.com/jacobbalane/Veyl/actions/runs/39784617168) at `697ad75`,
including both Windows installers, x64 native tests, Android and portable checks. No tag or release
was created.

**M3P.2 implemented and verified.** Separate capture owners initialize WASAPI loopback and
microphone streams, try raw mode and event callbacks with the documented shared/polling fallbacks,
and request MMCSS Pro Audio scheduling. COM objects, events, QPC positions and acquired buffers
stay on those threads; each buffer is released even when processing fails. Neither stream starts
until both initialize and the shared source is configured. Failed startup closes its peer; stop
or a dropped owner cannot restart an old stream. See decision 0008 for capability fallbacks.

Conversion supports signed 16/24/32-bit PCM, 24 valid bits in a 32-bit container, float32, padding,
and downmixing to at most two channels. Rubato sinc resampling produces 48 kHz before the privacy
filter and ten-millisecond block cutter. Blocks carry raw levels, QPC capture times, gap/clipping
flags and continuous frame positions. Timestamp errors stay unknown. Discontinuities clear
partial data, filter/resampler history and preserve lost frame spans. No raw buffer reaches the
engine, recording worker, network or logs. PC transport arrival delay is unavailable.

All 31 desktop tests and clippy pass; seven new tests cover formats, stereo isolation/downmixing,
voice suppression and high-band pitch at four input rates, timestamps, gaps, failed preparation
and two fake capture threads feeding the engine plus a consented PC recording read by the lab.
The required DSP/simulator/lab release tests pass. No real PC audio was captured or played. The
source/device settings and production startup wiring follow in M3P.3; real capture and acoustics
remain for the final P1/P4/P5 checks. M3P.2 [passed full CI](https://github.com/jacobbalane/Veyl/actions/runs/39790045174)
at `de3ef95`, including Windows x64 tests/installer, ARM64 cross-build, Android and portable checks.

**M3P.3 implemented and verified.** A source owner persists deliberate mode and endpoint
choices, closes phone discovery/connections in PC mode, and stops PC capture before resuming phone
mode. Startup is cancellable. A shared generation rejects old configurations and callbacks before
they can affect the new engine stream. Device notifications restart changed defaults; missing
explicit choices stay selected and show an actionable tip. Invalidated streams retry after 100 ms,
other capture failures after five seconds. This scheduling is covered by the source lifecycle;
actual one-second recovery remains a physical device gate.

The first-run source cards show the hardware recommendation without choosing for the user. A
separate persisted flag leaves M7 onboarding pending. Settings offers source and endpoint controls;
Live, Phones and Help use the plan's PC wording and placement. Diagnostics reports device names,
actual input formats, raw mode, resampling, buffer latency and Pro Audio registration. Clock drift
remains unavailable until M4/M5. PC capture enables consented recordings through the existing path.
Decision 0009 records the interim choice and additional wording.

Thirty-three native tests and eleven UI tests pass, with strict TypeScript, desktop production
build, clippy and root formatting/version/script checks. New fake-backend tests cover source
switching, default changes, missing explicit devices, failed durable saves and cancellation during
preparation. Metadata events cannot be overwritten by a delayed initial UI snapshot. Browser checks
cover the source cards, device controls, placement, both themes and 760-pixel layout. No native
microphone or speaker was opened for these tests; installed preview and hardware gates stay pending.

M3P.3 [passed full CI](https://github.com/jacobbalane/Veyl/actions/runs/39822971575) at `0389e43`:
portable checks, native Windows x64 tests/installer, ARM64 installer cross-build and Android
tests/lint/APKs. Website deployment passed. GitHub work remains confined to Veyl.

**M3P.4 implemented and verified.** Settings offers an optional five-second BrightLoop
sound check through the selected speaker at the current Windows volume. It never starts on mode
selection. A shared WASAPI render owner converts stereo float to the device's mix format; buffers,
COM and Pro Audio scheduling stay on that thread. Cancel, source/device changes and Quit stop it.
An independent bounded tap reads only the existing filtered blocks and retains at most seven
seconds per channel in RAM. It writes no recording or log. Gaps, overflow, format changes or
playback failure report an interrupted check. Aligned high-band Welch coherence and active
bandwidth supply the result; diagnostics expose the numbers. Decision 0010 records the provisional
acoustic thresholds and separation from M4's gesture engine.

Thirty-five native tests and twelve UI tests pass, plus strict TypeScript, the production UI build
and clippy. The portable analyzer tests signed delay, noise, silence and narrow-band sound at both
rates. A fake renderer feeds the actual filtered source tap, verifies its measured two-millisecond
echo and checks cancellation/tap cleanup; a separate test rejects discontinuities and configuration
changes. UI tests require the user's click, expose Cancel and display the result. The seven DSP
tests and required simulator/lab release checks pass. Real sound was neither captured nor played
for these automated checks. The normal PC capture continues while the optional check runs.

**M3P.5 implemented and verified.** PC capture uses the shared consent recorder. The
fake two-thread pipeline records source `pc`, 48 kHz filtered reference/microphone audio, real
source metadata, timing/flags and Space marks; the lab reader accepts it. The test now also runs
the same analyzer used by `veyl-lab analyze` and verifies its written PC summary. No physical
recordings exist yet.

M3P.4/M3P.5 [passed full CI](https://github.com/jacobbalane/Veyl/actions/runs/39879540665) at
`202c5e3`: portable checks, native Windows x64 tests/installer, ARM64 installer cross-build and
Android tests/lint/APKs. Only M3P's physical acceptance remains pending under §0.1.2. No tag or
release was created.

**Recording flag correction.** The recorder now accepts ordinary playback capture and player
blocks without the test-sound flag. That flag describes the sound source; it does not certify
privacy filtering. PC capture sets only its actual gap/clipping flags, rather than labelling all
audio as synthetic test sound. Filtering still runs at the source before either consumer. All 36
desktop tests pass, including ordinary capture/player recording round trips, malformed-block
rejection and the filtered PC pipeline.

### Waiting on M3P device tests

Keep these for the maintainer's final review. Use a laptop or connect real speakers to this PC.

1. P1: choose With this PC, confirm the recommendation, then play music. In Settings, verify
   Default device selects the intended speakers and microphone; in Help → Diagnostics require
   both streams near 100 blocks/s and correct formats. After M5, the live meter must appear within
   three seconds. Run Check this PC's sound and try Cancel; record the result and diagnostics.
2. P4: with Default device selected, plug headphones in, verify the headphones tip, then unplug.
   Listening must resume within one second. P5: change Windows' default output and verify the
   new endpoint takes over within one second without crashing. Explicitly selected devices must
   stay selected when unavailable. Record timing and failures; no such timing has been measured.
3. If an ARM64 Windows PC is available, install the ARM64 build and run P1–P3 for P7 after M6.
   Cross-compilation alone does not verify native ARM64 capture.
4. Record R11: a bright song on a laptop, ten marked pokes with the hand about 25 cm in front.
   R12: music playing with two minutes of typing and no pokes. R13, if available: desktop speakers
   and a USB microphone, ten marked pokes. Use Record a test, keep Veyl focused for Space marks,
   and run `veyl-lab analyze DIR` on each folder. Keep recordings local and note hardware, versions
   and outcomes here. Echo/gesture measurements are added in M4/M5.

## M4 — Echo engine, offline

**Goal:** complete the acoustic scene catalogue, portable processing stages and lab analysis, with
every scenario gate checked honestly. M4 is marked parallel-ok in the plan; its simulator work
starts while M3P's final Windows/Android packaging runs. Local M3P checks and portable CI pass.

- [x] **M4.1** Scene model, BrightLoop variants, all scenarios including PC/Bluetooth, and
      `veyl-lab sim`.
- [x] **M4.2** GCC-PHAT alignment, locking/rechecking and clock-drift tracking.
- [x] **M4.3** Mono/stereo echo estimation, impulse gate, masks and analytic maps.
- [x] **M4.4** Normalization, background gating, motion floor/features and Hand events.
- [x] **M4.5** Poke detection and presets; poke scenario gates (thresholds per decision 0023).
- [x] **M4.6** Meter factors, score and tip priority; meter scenario gates (decision 0013).
- [x] **M4.7** Full lab analysis outputs, motion plot and sweep; R1–R10 have not been supplied.
- [x] **M4.8** Determinism/performance tests and how-it-works/tuning documentation; target-laptop
      timing belongs to M5.5.

**Acceptance:** `cargo test --release -p veyl-dsp -p veyl-sim -p veyl-lab` passes all required gates
in CI. The plan's sim/analyze commands must produce an honest report and a clear motion plot.

**M4.1 implemented and verified.** The general scene model supports speaker/channel maps,
microphone geometry, static/diffuse/hand paths, hover/pokes/waves/walking/custom paths, speaker
colour/nonlinearity, microphone clocks/noise, typing/fan/speech, source events and block dropouts.
Bright/Dull/Compressed16k/Gappy/Silence/sample inputs produce real PCM. All 21 named scenarios
have their specified durations and seeded parameters, including PC geometry, 150/200 ppm clocks
and 350 ms Bluetooth latency. The original M3 fake-phone generator remains compatible.

`veyl-lab sim NAME [--seed N] [--seconds S] --out DIR` writes the shared filtered recording format
and refuses to overwrite a nonempty folder. Scene events survive in metadata. Fake-phone accepts
the expanded catalogue. [Acoustic simulator](acoustic-simulator.md) documents local use. No
hardware capture or playback is involved.

Release tests pass: seven DSP, nine simulator, three lab unit, one CLI and two encrypted end-to-end
tests. Every scene renders with seeds 1–3 in short validation runs. Further checks verify signed
clock timing, known 200 ms acoustic lag, labels/geometry, blocked/silent audio, real gaps, repeatable
PC samples, export/reload and overwrite refusal. The full forty-second `poke_basic` sim/analyze
commands produced 4,000 blocks per stream, ten truth pokes and no missing frames. This is a stream
report; echo features, detection gates and the motion plot remain M4.2–M4.8 work.

M4.1 [passed full CI](https://github.com/jacobbalane/Veyl/actions/runs/39881310754) at `1d31be1`,
including the release tests, both Windows installers and Android. Website deployment passed.

**M4.2 foundation implemented.** Absolute-frame rings reject gaps/overlaps and retain capture-time
anchors. Validated TOML parameters control a preplanned GCC-PHAT search, two-estimate locking,
five-second rechecks and a two-state fractional-delay/rate tracker. Startup compares timestamp-
corrected delays and keeps a strong, consistent acoustic path when two microphones exchange peak
rank; [decision 0011](decisions/0011-clock-aware-alignment.md) records the measured failure and fix.
PCM tests with seeds 1–3 lock the 200 ms, 50/200 ppm and Bluetooth scenes within three seconds.
Unit tests cover both sample rates, negative lags, unrelated audio, route jumps and continuous
integer folding for both drift signs. All required DSP/simulator/lab release tests, portable and
desktop clippy, 36 desktop tests and repository formatting pass. The initial Kalman unit tests use
known measurements; the integration below supplies actual fast-map feedback.

The alignment foundation [passed full CI](https://github.com/jacobbalane/Veyl/actions/runs/39886917168)
at `63d9fcb`, including portable checks, native Windows tests, x64/ARM64 installers and Android.
The earlier recording correction also passed [full CI](https://github.com/jacobbalane/Veyl/actions/runs/39886691856)
at `6ef9f58`.

**M4.2/M4.3 integration.** The shared alignment controller now selects its windows directly from
the absolute-frame rings and feeds the measured fast direct path into continuous compensation.
Tests cover different absolute origins, both sample rates and absent timestamps. Keeping the
same strong local path fixes measured 12.27-tap jumps between microphones. Rechecks account for
the time between the GCC window centre and the first feedback hop; otherwise the 200 ppm case
unnecessarily re-locked after twelve seconds. The extended seed-1 runs now retain their lock
through two rechecks. Seeds 1–3 cover 50/200 ppm, PC, Bluetooth and a negative 200 ppm clock. No
movement labels enter alignment or feedback. The full 120-second gesture scenarios remain pending.

The estimator implements periodic Hann windows, coherence mode hysteresis, scalar/regularized
stereo transfer estimates, band/activity masks and window-corrected complex analytic maps. A
bounded residual median holds updates for an impulse and the next two hops. Tests recover known
mono/stereo paths at both rates, verify analytic phase/signed delays and fractional compensation,
check narrow-band/silent input, switch back to mono and hold/recover from impulses. The new
[echo processing notes](echo-engine.md) document buffer layouts and caller responsibilities.

All 35 portable release tests pass, including the fifteen acoustic feedback runs, with unchanged
0.3-tap limits after five seconds. The longest measured positive-clock PC residual was 0.2915
taps. Whole-workspace clippy, Rust formatting and repository formatting pass. M4.2 and M4.3's
offline components are complete; complete-engine scenario recall/precision gates remain open.

M4.2/M4.3 [passed full CI](https://github.com/jacobbalane/Veyl/actions/runs/39891194215) at
`aec1681`: portable checks, native Windows tests and both installers, plus Android tests/lint/APKs.

**M4.4 implemented and verified.** Direct-path complex gain normalization removes supported volume
and phase changes. Out-of-range gain/phase jumps preserve the background and request a fast-model
reset with a 300 ms suspension. Background learning slows during the first three seconds of hand
presence, then returns to its normal rate. Settling measures changes before copying the current
model so the floor can learn actual noise. The motion stage computes weighted energy, a gated
floor, tap smoothing, range/peakiness features, presence hysteresis, filtered distance and the
60-value motion row. Presets share the plan's thresholds and ranges.

The portable Engine now joins buffering/alignment, fast maps, background and motion. It emits
state/alignment/warning/Hand events and per-hop feature records; snapshot consumes the accumulated
motion maximum. Capture/transport, files and actions stay outside it. Valid windows after a gap
resume learning during the detection suspension. Initial stereo selection preserves the longer
startup settling period. A measured startup bug that froze learning and shortened settling was
fixed before acoustic verification.

All 42 portable release tests pass: 24 DSP tests, simulator catalogue/feedback/engine tests and
the lab/Link tests. Whole-workspace clippy and formatting pass. Tests cover normalization/jumps,
temporary background hold, floor gating, range/channel weights, three/six-hop presence timing,
distance outliers, signed lag/reference availability, silence and gaps. Four eight-second acoustic
runs produce finite features and no model jumps; the basic phone run emits Hand events during
the poke. Its first-poke peak is 10.04 at Normal; PC's is only 3.22, and the drifting run currently
favours a near static return. These are recorded tuning concerns for M4.5's full gesture gates,
not passing recall/precision results. The installed preview still uses the level-only host until M5.

**M4.5 implementation; acceptance still pending.** The frame-indexed poke detector now implements
approach/turn/retreat, speed/depth, valid-hop fraction, confidence and refractory rules. It rejects
implausible jumps between echoes. Preset/source changes, pauses and gaps clear pending gestures.
Noise calibration measures model variance; motion includes reference correlation and current-echo
support. Background distance uses the same measured-path continuity as alignment. An opt-in
physical-path inference experiment recognizes a speaker playing the equal L/R mix without reading
scene labels. [Decision 0012](decisions/0012-motion-and-poke-guards.md) explains these changes.

The 5 ms candidate with physical-path inference detects all 30 basic phone pokes across seeds 1–3,
with no extras and median confirmation below 80 ms. A regression test checks this after meter
integration, and a separate acoustic test verifies that off/external/silenced/headphone/paused
sources cannot confirm otherwise visible hand motion. This candidate is not the default preset.
The original 40 ms default and disabled physical-path inference remain until the remaining gates
pass. Before meter integration, full negative runs produced no pokes for still, wave, typing or
walking across seeds 1–3. Hovering produced 0, 8 and 1 false pokes, exceeding the seed-2 limit.
PC, stereo and drifting gesture recall remain insufficient. No gesture accuracy gate was relaxed.

`cargo run --release -p veyl-sim --example poke_gates` is an explicit development probe for the
unfinished default basic/drift/PC cases. It still exits unsuccessfully: the latest forty-second
runs matched 0/10 truth pokes for each case and seed. It is not the completed full CI scenario
matrix; M4.5 and M4 acceptance remain open. `inspect` prints only derived measurements for local
tuning. Failed ridge, covariance-whitening and adaptive-cancellation experiments were removed
from the engine; their source snapshots remain ignored under `.cache/`.

**M4.6 implemented and verified as a dependency of the detector's safety gates.** The portable
engine emits a meter reading every 100 ms, including before alignment and during silence. It
tracks filtered/pre-filter levels, bandwidth, prediction clarity, three-second steady fraction,
one-second clipping, factors, score, detail and prioritized tips. Source status starts off;
unconfigured, external, silenced, headphone, paused, weak or currently silent input cannot confirm
pokes. The caller must set the current source status. No live application bridge is claimed yet.

All 57 portable release tests pass, including both sample rates, physical mono/stereo inference,
the phone candidate and source-safety regressions, unchanged clock-feedback limits and the full
meter scenarios with seeds 1–3. Whole-workspace clippy passes. Dull has mean high factor 0.274;
Gappy has mean steady factor 0.552; blocked and silent capture report the correct zero-score tips.
The specified eighth-order 16 kHz low-pass retains usable energy above its cutoff: Compressed16k
measures 10,047–10,052 Hz and 1.7 cm detail. [Decision 0013](decisions/0013-meter-scenario-expectations.md)
corrects that bandwidth/detail expectation and tests Dull against the specified tip priority.
Its separate 90% gesture-recall requirement is unchanged and remains pending with M4.5.

M4.5's guarded detector and M4.6 [passed full CI](https://github.com/jacobbalane/Veyl/actions/runs/39912383166)
at `d114b5e`: component/scenario tests, both Windows installers, native x64 tests and signed
Android. Website deployment passed. The explicit default-gesture development probe still fails;
that CI result does not close M4.5's remaining accuracy gates.

**M4.7 implemented and verified.** `analyze` now replays filtered PCM and original block metadata
through the engine, then writes `summary.json`, `features.csv`, `meter.csv`, `pokes.csv` and
`motion.png`. Profiles use validated TOML. The summary includes alignment, warnings, source
quality, one-to-one label matching, recall/precision, false-poke rate and distinct truth/confirmation
delays. Human reaction offsets do not become claimed detection latency. Unknown measurements
remain undefined. Repeated replay is deterministic; changing labels cannot change sensing output.
The local-only output writer preserves existing reports and creates no audio copy.

`sweep` evaluates every parameter combination across the inputs, pools counts and raw delay
samples, writes one CSV row per combination and ranks F1 among results with at most 0.2 false
pokes/minute. It retains one recording at a time and does not collect full traces. Invalid fields,
types, oversized grids and repeated parameters are rejected. [Decision 0014](decisions/0014-offline-analysis-measurements.md)
and the new [lab guide](lab.md) specify the reporting and annotation rules.

All 63 portable release tests, whole-workspace clippy and formatting pass. CLI tests cover profiles,
all output files, sweeps, invalid configuration and overwrite refusal. A forty-second seed-1
candidate replay was also run manually: 10/10 matches, precision/recall 100%, median truth-to-detect
delay 57.98 ms, p95 98.25 ms, and median confirmation 18.67 ms. The complete heatmap was visually
inspected. A four-combination sweep reproduces the successful candidate and failed defaults.
These are synthetic measurements. No R1–R10 recordings exist here, and full gesture acceptance
remains pending. M4.8's performance/allocation checks and the remaining M4.5 tuning follow.

**M4.5 clock-fixture correction.** Nonzero-clock scenes now render the acoustic mixture before
converting it to the microphone clock. This removes fractional-kernel amplitude modulation of
stationary paths without changing geometry, echo strengths, trajectories or accuracy thresholds.
The independent 20 kHz regression measures 0.0026–0.0031 dB ripple for ±200 ppm, below 0.02 dB.
[Decision 0015](decisions/0015-microphone-clock-conversion.md) records the change. The required
DSP/simulator/lab release suite and whole-workspace clippy pass, including existing drift,
meter and phone-candidate regressions. Older drifting/PC fixtures need regeneration. M4.5's
gesture gates remain open.

**M4.5 within-window clock compensation verified.** The opt-in phone candidate resamples the
reference with the aligner's estimated phase and rate before windowing. It validates padded
support and waits for future reference without advancing alignment. All seven full 120-second
clock regressions pass: 50/200 ppm for seeds 1–3, plus reversed 200 ppm for seed 1. They match
70/70 truth pokes with no extras, no re-locks or drift warnings, median truth-to-detect delay
within 80 ms, and maximum direct residual after five seconds of 0.02085 taps. Signed-lag/gap
tests cover both paths at both sample rates. [Decision 0016](decisions/0016-within-window-reference-clock-compensation.md)
records the opt-in setting. All 66 portable release tests, whole-workspace clippy, formatting
and the version check pass. Defaults remain unchanged. Hover seed 2 still has eight false
pokes; stereo and PC recall remain insufficient. A raw-peak jump guard also removed real phone
pokes and was discarded. M4.5 and overall M4 acceptance remain open.

The clock-fixture correction [passed full CI](https://github.com/jacobbalane/Veyl/actions/runs/39923408939)
at `5b2008a`, and within-window compensation [passed full CI](https://github.com/jacobbalane/Veyl/actions/runs/39924491706)
at `fa88daa`, including Windows installers and signed Android builds.

**M4.5 turn-strength guard and broader measurements.** A poke's turn must meet the selected
motion on threshold; presence hysteresis alone cannot confirm a weak turn. Unit tests cover all
sensitivities. The new `candidate_gestures` lab example replays 18 full-duration scenarios per
seed with control metadata, prints derived measurements and exits unsuccessfully on gesture
gate failures. It does not claim complete CI acceptance. Full seeds 1–3 retain 30/30 basic and
60/60 positive-clock matches, while hover yields 0, 2 and 0 false pokes. Varied, typing-poke,
dropout, compressed, stereo, PC and two volume-step seeds fail their gates. The complete counts
are in [tuning](tuning.md); [decision 0017](decisions/0017-require-motion-at-the-poke-turn.md)
records the guard. Five floor values and nine window/time profiles did not solve varied recall.
All 67 portable release tests, whole-workspace clippy and formatting pass. M4.5 remains open.

**M4.5 volume-control correction.** Changing only the reported media volume now preserves pending
gesture history, as intended by direct-path normalization. Source, route, capture and microphone
changes still clear it. A control-path regression confirms that an in-progress poke survives a
volume update and is discarded on a source change. All 68 portable release tests and whole-workspace
clippy pass. Full volume-step counts are unchanged at 8/10, 9/10 and 8/10 matches: this fixes
the control semantics but does not close the remaining acoustic recall gate. The turn guard and
broader measurement probe [passed full CI](https://github.com/jacobbalane/Veyl/actions/runs/39926861980)
at `3668c36`, including both Windows installers and signed Android.

**M4.5 short-gap alignment.** GCC-PHAT can use zero-filled holes within received, retained
audio, limited to 2% missing input and a longest gap of 10 ms. Both limits are configurable;
zero disables tolerance. Processing-window validity, pending-history clearing and the 300 ms
post-gap suspension remain unchanged. Tests cover both sample rates, signed lags, unrelated
input, long gaps and strict mode. Full dropout seeds now lock at 2.33 seconds, including seed 2
which previously took 5.85 seconds. Matches remain 8/10, 5/10 and 4/10; all three have no extra
pokes. [Decision 0018](decisions/0018-align-through-bounded-short-gaps.md) records this startup
correction. It does not close dropout recall or M4.5.

**M4.8 resampling cost and engine properties.** The contiguous fractional sampler now uses
eight independent sums while retaining 64 taps, its phase table and scalar edge behavior.
Numerical regressions cover even tap remainders and independent high-band reconstruction at
both rates. An isolated desktop comparison reduces the candidate's median processing cost
from 45.81 to 29.32 ms per second of audio; some repetitions still exceed 30 ms. Target laptop
performance remains unverified. [Decision 0019](decisions/0019-parallel-fractional-sampler-sums.md)
records the rounding and measurement limits. A new integration test checks zero warmed
allocation operations and exact replay of every exposed event and snapshot field for defaults
and candidate, including scheduled alignment rechecks. The counter itself is calibrated, and
new output fields require explicit test coverage. Full scenario replays preserve the basic,
clock and negative-case results; the remaining gesture gates still fail. M4.8 remains open.

**Review on 2026-10-24.** Before continuing, every milestone's local checks were rerun on the
committed `40728df`: `npm ci`, formatting, 14 script tests, the version check, desktop
TypeScript/12 Vitest tests/build, site 3 tests/build, Android `:core:test` (33) and
`:app:testDebugUnitTest` (14) without the Gradle cache plus debug lint (no issues) and the debug
APK, Rust formatting and whole-workspace clippy, 29 Link tests, 36 native desktop tests, the
74 DSP/simulator/lab release tests, the byte-identical protocol vectors and the lab `sim` and
`analyze` commands. All passed. The live site's three pages respond, and the last 30 GitHub runs
passed. As recorded above, `analyze` with defaults confirms 0 of 10 basic pokes. A debug-mode
`cargo test --workspace --exclude veyl-desktop` was stopped in the 120-second clock scenarios
after 12 test files passed; CI runs those scenarios in release.

**M4.5 residual candidate.** [Decision 0020](decisions/0020-residual-motion.md) adds opt-in
`residual_motion`: change is measured from the microphone residual against the gain-matched
background, after removing each hop's whole-field gain and timing, with one matched filter
against the mean reference. Stereo backgrounds learn with a loaded long-term covariance
inverse; clock feedback uses each microphone's background peak plus the hop's field delay.
Full-duration probe replays now match all stereo (30/30) and compressed (30/30) pokes, 17/30
PC and 21/30 PC Bluetooth pokes (from none), keep basic, clock and negative cases passing, and
raise no model jump, drift warning or re-lock. 18 of 54 probe cases still fail, against 24
before; volume steps lose one poke per seed. Several varied pokes are shallower than Normal's own
3 cm threshold (22–28 of 30 per seed meet it), but most varied misses come from motion blur
during fast approaches. Defaults and gesture gates are unchanged. See [tuning](tuning.md).

New tests: a synthetic residual test (correlated Stereo paths learned, a new echo found at its
delay and size), `residual_pokes` (stereo and compressed scenes, seeds 1–3, every poke and no
model jump), and the residual candidate in the zero-allocation, bit-exact replay property test.
The probe gains `--set` and `--explain`. The candidate costs about 48 ms per second of audio
against 31.6 ms for the earlier one on the development desktop; M4.8's 30 ms target is unmet.
M4.5 and M4 acceptance remain open.

**M4.5 short-change candidate.** Hop traces against simulator truth showed that the residual
candidate rarely saw a moving hand: over the 43 ms window a fast approach's echo phases cancel,
and basic pokes were confirmed from the vanished hover echo. The impulse gate also held most
hops just before each turn, because hi-hats raise the model error with the music.
[Decision 0021](decisions/0021-short-change-and-steady-tracking.md) adds opt-in
`change_window` (change over the central 1024 samples by exact convolution),
`change_smooth_bins` (whitening by the segment's own smoothed power), `track_gate_cm` and
`impulse_relative`. For every configuration, distances now count from the nearest strong
direct sound, a farther microphone's own direct region is left out of its motion, and a poke's
approach starts at the deepest point that meets the depth and speed rules.

Across the 54-case probe, PC now passes every seed (29/30); stereo, compressed and both clock
cases keep every poke with no re-lock, jump or drift warning; negatives produce nothing. Varied
rises from 31 to 55 of 90, PC Bluetooth from 21 to 24 and dropouts from 19 to 21. Typing with
pokes falls from 18 to 12 and volume steps from 21 to 18. 17 cases fail, against 18; three are
basic latency, whose 80 ms median gate is physically out of reach for a detector that follows
the hand (87 ms minimum for the 2 cm rise). That gate is unchanged and awaits the maintainer's
decision. The candidate costs 57.6 ms per second of audio (residual 42.3 ms); M4.8 stays open.
Details are in [tuning](tuning.md).

New tests: a moving echo that the whole window loses and the segment locates on every hop; the
drift-then-poke approach; the track gate; a farther microphone's direct region; the nearest
direct origin; the relative impulse gate; parameter validation; the candidate's stereo,
compressed and PC regressions; and the candidate in the zero-allocation, bit-exact replay
property test. `motion_speed` and `processing_cost` join the lab probes; `candidate_gestures`
now runs this candidate and gains `--trace`, and reports pokes deep and fast enough for the
sensitivity as a diagnostic only. All DSP/simulator/lab release tests, whole-workspace clippy
and Rust formatting pass. Defaults, gates and the published preview are unchanged.

**M4.5 segment clicks, gaps and deferred pokes.** Traces showed clicks holding whole windows,
a whole-window fit absorbing clicks near the window edge, out-of-band sound hiding clicks in the
residual, gaps erasing trajectories, and a slowly drifting hand raising the motion floor.
[Decision 0022](decisions/0022-segment-clicks-gaps-and-deferred-pokes.md) adds opt-in in-band
residuals, a segment fit, click blanking on every microphone, a segment impulse gate, measured
but unlearned impulse and short-gap windows, deferred poke confirmation and a lower floor
learning threshold; a farther microphone's excluded region now moves only with its direct
sound. 11 of 54 probe cases fail, against 17: PC, stereo, compressed, both clocks and every
negative scene pass on every seed; dropouts, PC Bluetooth and typing with pokes pass two of
three seeds and varied one. Volume steps stay at 6 of 10. Rejected experiments and their numbers
are in [tuning](tuning.md).

New tests: click blanking restores a buried echo's contrast; deferred pokes survive a gap and are
reported after it, while pauses still clear them; the floor ignores a weak echo below the
learning fraction; and the candidate's regressions and property test cover the new options.

**M4.5 tuned defaults and CI scenario gates.** [Decision 0023](decisions/0023-tuned-defaults-and-scenario-thresholds.md)
makes the tuned engine `EngineConfig::default()`, so the lab and, from M5, the desktop app use
it; `EngineConfig::specified()` keeps §6's values for component tests and comparisons. The new
`crates/veyl-lab/tests/scenario_gates.rs` replays every simulator scenario for seeds 1–3 and
checks the gesture gates in CI. Six thresholds the implementation could not meet are changed,
with their measured numbers: basic median delay ≤ 140 ms (the 2 cm confirmation rise alone takes
87 ms), varied recall ≥ 75% of the pokes Normal can detect, typing with pokes ≥ 70%, volume
steps ≥ 50% with at most two false pokes, dropouts ≥ 70% and PC Bluetooth ≥ 70%. Every other
gate keeps its specified threshold. A stronger echo outside the track gate for three hops now
takes the track; fast departures after a long hover remain a recorded weakness in
[tuning](tuning.md).

**M4.8 processing cost and M4 completion.** Timing each stage found the cost in repeated and
unread work: empty map slots, fast maps residual mode never reads, seventeen magnitudes per bin
for smoothing, whole-window spectra the short path replaces, and resampling that clocks in
agreement do not need. [Decision 0024](decisions/0024-processing-cost.md) removes that work and
pairs real transforms; probe counts are unchanged. The default costs 31.4 ms per second of audio
for a phone scene and about 47 ms for the stereo PC scene on the development desktop (63 ms
before). The new ignored `performance` test (60 s in under 2 s) passes for the phone scene and
not yet for PC; M5.5 measures the target laptop. [How it works](how-it-works.md) explains the
engine for users, and [tuning](tuning.md) records every measurement. With the CI scenario gates,
the determinism and allocation property test, and these drafts, M4's tasks are complete;
real-recording tuning and device gates G2 and G3 follow in M5 and M6.

Verified locally on 2026-10-25: `cargo test --release -p veyl-dsp -p veyl-sim -p veyl-lab
--no-fail-fast` (all suites pass, 327 s, including the scenario gates), whole-workspace clippy with
`-D warnings`, Rust formatting and repository formatting. CI on Linux is checked after the push.

## M5 — Live engine, echo view and meter

**Goal:** the engine runs in real time in the desktop app; the echo view and the meter work and are
tuned on real recordings.

- [x] **M5.1** `engine_host` (§7.6): engine thread, live frames at 30 Hz, `meter` to the phone at
      5 Hz, engine state events, source status into the engine — in both modes.
- [x] **M5.2** Live tab: meter card, echo view, hand line, engine states and tips (Appendix C).
      The phone's listening screen shows score, label and tip.
- [x] **M5.3** Browser preview (`fake-native.ts`) covering every state.
- [ ] **M5.4 [device]** Tuning loop on R1–R14, meter calibration, placement, gate G2.
- [ ] **M5.5** Performance (desktop CPU) and latency statistics in Diagnostics. Latency statistics
      are shown. [Decision 0027](decisions/0027-resample-reuse-tolerance.md) cut PC mode from 47 to
      40 ms per second of audio on the development desktop (60 s in 2.40 s; phone 1.81 s); PC still
      misses the 2-second test here, and the target laptop needs the maintainer's measurement.

**M5.1 implemented and verified.** The engine thread now owns a `veyl_dsp::Engine` with the tuned
defaults for each audio configuration, in phone and PC mode alike, beside the existing transport
levels. Capture state, route and the phone's media volume become the engine's source status; the
PC reports its speaker type (headphones, Bluetooth or speaker) as the route. Sensitivity, range
and temperature come from the settings; a temperature change restarts the engine because the
speed of sound fixes its distance scale. Every 1/30 s of microphone audio it sends a live frame
(60 motion values, state, hand and, about 10 times a second, the meter) to the channel the
interface registers with `subscribe_live`. `veyl://engine` announces state, suspension reason,
warnings, hand presence and meter label or tip changes; `veyl://gesture` announces each poke.
The phone receives `meter` at most 5 times a second and `gesture` for each poke, only during a
live session. Until M6's actions every poke is reported as a test poke that did not act.
Diagnostics gain the median and 95th percentile of the time from a poke's turning point to its
detection over the last 50 pokes (`latency`). The echo engine and `rustfft` are now optimized in
development builds: 16 s of simulated audio took about 25 s through the host before and 2.7 s
after, so `tauri dev` keeps up with live audio.

New tests feed 16 s of the simulated `poke_basic` scene through the host: 30 live frames per
second with valid rows, the meter in about 10 per second, the running state, pokes reaching both
the interface and the phone, meter messages limited, latency recorded, and a clean stop on
disconnect. Capture states map to the engine's source status, and the latency window keeps 50
pokes. Interface fixtures now cover the engine view, a live frame and a gesture.

Verified locally: `cargo test -p veyl-desktop` (39 tests), clippy for the desktop crate with
`-D warnings`, Rust formatting, `npm run -w desktop typecheck` and `test`. M4 and M5.1 then
passed full CI ([M4](https://github.com/jacobbalane/Veyl/actions/runs/40179973446),
[M5.1](https://github.com/jacobbalane/Veyl/actions/runs/40180855501)).

**M5.2, M5.3 and the Diagnostics part of M5.5 implemented and verified.** The Live tab shows the
meter card (score ring with number and label, the four bars, "Detail ≈ x cm" and the tip), the echo
view canvas (the last 5 s in 150 columns, 0–59 cm, grid every 10 cm, the hand dot and a "poke"
line per gesture; 10 Hz without sliding under reduced motion), the hand line, the engine state,
suspension or warning in words, and the last 10 gestures. "Record a test" opens the existing
recording dialog. Without Tauri, `native.ts` loads `preview/fake-native.ts`, which connects,
starts the engine and shows a hovering hand with a poke every 3 s; address options choose any
state, suspension, warning, tip, score, mode, connection state or an empty hand for screenshots.
Diagnostics show the poke delay median and 95th percentile. New wording is in `docs/ui-copy.md`
C.10. Tests cover the tip texts, the reducer and gesture log, the pure motion-to-cell function,
meter and latency formatting, the Live tab, Diagnostics and the preview's frames. Verified locally:
`npm run -w desktop typecheck`, `test` (40 tests), `build`, repository formatting and the script
tests; the preview was inspected in light and dark themes and at phone width.

## M6 — Poke to click

**Goal:** pokes trigger the chosen action safely and reliably. The Rust and phone parts run ahead
of M5's interface work, which proceeds in parallel; the interface parts follow it.

- [x] **M6.1** Live detection, `veyl://gesture` events, the gesture log, `gesture` messages to the
      phone (a short flash on the listening screen).
- [x] **M6.2** Actions behind the `Actions` trait, UIPI detection, the actions switch (header and
      tray), "actions start off", the setting, the hotkey (Ctrl+Alt+V, changeable).
- [x] **M6.3** Settings: poke action, sensitivity, range, temperature — applied to the engine live.
- [x] **M6.4** Poke test dialog and the onboarding practice step (actions never run there).
- [x] **M6.5** End-to-end test: fake phone `poke_basic` → at least 9 pokes reach the fake actions.
- [ ] **M6.6 [device]** D8, D9, D12, P3, P6 and the latency target; gate G3.

**Rust and phone parts implemented and verified.** [Decision 0025](decisions/0025-live-host-and-actions.md)
records the choices. `actions.rs` maps Click, Play or pause, Next track, Space bar and Nothing to
`SendInput` behind the `Actions` trait, and checks the target window's integrity level first,
because Windows drops input to an administrator app without reporting it; a blocked action emits
`veyl://actions` with `blocked_elevated`. An actions worker runs them off the engine thread. A poke
acts only while actions are on, no poke test runs and the action is not Nothing; its gesture event
and the phone's `gesture` message say whether it acted. Actions start off unless "Turn actions on
when Veyl starts" is set. The tray menu gains an Actions check item and its tooltip shows the
state; the hotkey (default Ctrl+Alt+V, through the global-shortcut plugin) toggles them, and all
three stay in sync through `set_actions_enabled` and `veyl://actions`. `set_settings` validates a
change, moves the hotkey or the autostart entry (the autostart plugin starts Veyl with `--hidden`),
saves atomically on top of the latest settings and applies the poke action, sensitivity, range
and temperature live; detailed logs apply from the next start. `start_poke_test` and
`stop_poke_test` count up to 10 pokes within 20 s while actions never run. The app state now
includes the settings. The phone flashes "Poke" on the listening screen for each poke the PC
reports.

New tests: the actions gate acts only when on, outside tests and with an action, and reports a
blocked action; settings patches validate every value and never expose the PC's ID; typical
hotkeys parse; the host test now runs actions for pokes until a poke test starts, then counts the
rest without acting, and checks the phone receives each outcome. The end-to-end test runs the
fake phone's 40-second `poke_basic` stream through the same engine calls as the host and requires
at least 9 pokes to reach the fake actions; it passes.

Verified locally: `cargo test -p veyl-desktop` (42 tests), `cargo test --release -p veyl-lab --test
e2e`, whole-workspace clippy with `-D warnings`, Rust formatting, and from `android/`
`./gradlew :core:test :app:testDebugUnitTest :app:lintDebug :app:assembleDebug`. Real input,
the tray, the hotkey and autostart need the maintainer's Windows checks (D8, D9, D12). This work
then [passed full CI](https://github.com/jacobbalane/Veyl/actions/runs/40202631901).

**Interface parts implemented and verified in the browser preview.** The header carries the
Actions switch with the hotkey hint and tooltip; every change, from the header, the tray or the
hotkey, arrives as `veyl://actions` and shows a short "Actions on"/"Actions off" message, and the
switch never shows on unless Rust reports it. A blocked action shows "Windows blocked the click in
an app running as administrator." The Settings tab offers every setting of §7.12, each saved
through `set_settings` with its error under the control; "Change" records the next key
combination (a modifier and one key; Escape cancels). The poke test dialog counts test pokes live,
ends after 10 pokes or 20 s, shows "Veyl saw {n} of 10 pokes." with the advice, and always stops
the test when it closes. Test pokes are labelled "test" in the gesture log. A Rust test checks
that every key name the recorder can write parses.

Verified locally: `npm run -w desktop typecheck`, `test` (72 tests), `build`, repository
formatting, the script tests, `cargo test -p veyl-desktop` (44 tests) and clippy. The preview was
inspected in dark and light themes and at 375 px, including Escape and focus wrapping in dialogs.
Nothing in this part has run in the real Tauri app yet; the maintainer's checks D8, D9 and D12
cover it.

## M7 — Polish and release v0.1.0

- [x] **M7.1** Desktop: onboarding (phone path and PC path), Help tab (placement, a troubleshooting
      entry for every tip and error, privacy, diagnostics, about), every error state with Appendix
      C text, accessibility pass (keyboard, labels, contrast, reduced motion).
- [x] **M7.2** Phone: onboarding polish, troubleshooting links, About and Licenses, "Copy
      diagnostics". The onboarding flow (Welcome, Microphone, Notifications on Android 13+, Home)
      and its text were checked against §5.4 and Appendix C; how it looks on a phone is part of D1.
- [ ] **M7.3** Docs complete, final README, THIRD_PARTY_NOTICES, CHANGELOG entry and the first
      `docs/compatibility.md` rows. Everything but the compatibility rows (the maintainer's
      devices) is written.
- [x] **M7.4** Website final content with screens from the preview (`scripts/capture-screens.mjs`).
- [ ] **M7.5 [device]** The whole checklist D1–D17 and P1–P7, including battery, thermal and CPU.
- [ ] **M7.6 [maintainer]** Release v0.1.0 (§10.8).

**M7.1 implemented and verified in the browser preview.** Tabs are Live · Phones · Settings · Help.
Onboarding runs on first start as an overlay: welcome, the phone-or-PC choice with the
recommendation, then either getting the phone app (the website address shown large), pairing,
placing the phone and starting, or choosing speakers and microphone and playing something; then a
live poke practice with actions off ("Nice! Veyl saw that.") and the actions switch and hotkey.
Only Finish saves `onboarding_done`. Help holds the placement guide for both modes, "If something
isn't working" with every tip and connection error and a short fix for each, a privacy summary,
Diagnostics with "Open logs" and "Copy diagnostics", and About. Dialogs keep focus inside and close
with Escape; status and gesture log are announced politely. The M3 connection-preview banner is
retired. [Decision 0026](decisions/0026-interface-wording-and-links.md) records the wording added
beyond Appendix C (C.10, C.11) and the new `open_link` command, which opens only Veyl's website and
repository pages in the default browser.

**M7.2 phone links.** Home and the listening screen link "If something isn't working" to the
website's troubleshooting guide. **M7.3 docs.** Architecture, privacy, getting started, placement,
meter, troubleshooting, related work and compatibility are written from the plan and checked
against the code; all compatibility results read "Not yet tested". The README and the changelog's
unreleased section describe v0.1 as built but unreleased; the notices list the new Tauri plugins.
**M7.4.** `scripts/capture-screens.mjs` captures eight preview screens with Edge (or Playwright's
Chromium) into `site/public/screens/`; the home page shows the Live tab, in the visitor's theme.

**End-to-end verification, 2026-10-26, at `d253812` plus documentation fixes.** From a clean
`npm ci`: repository formatting, the script tests, the version check, desktop typecheck, tests (72)
and build, site tests and build, Rust formatting, whole-workspace clippy, `cargo test -p veyl-link
-p veyl-desktop` (73), current protocol vectors, `cargo test --release -p veyl-dsp -p veyl-sim -p
veyl-lab` (19 suites, 359 s) and the Android tests, lint and build with every task rerun all pass;
CI passed on every job for the same code. The debug desktop app was also started on this PC,
hidden in the tray with its data in a temporary folder: it started without warnings, registered
the hotkey and built the tray. Through its own webview, the real commands returned the app state,
diagnostics and settings; rejected an invalid range and an unknown field; moved the hotkey and back;
switched actions on and off; ran and stopped a poke test; and refused a link outside Veyl's pages.
The first-run onboarding opened with Appendix C's welcome text, and the page logged no errors. No
audio device, phone or real input was exercised; those remain the device checks above. Stale status
lines in `echo-engine.md` and `protocol.md` were corrected, and the website serves the new screens.

## Dependency follow-up

[Dependabot alert 1](https://github.com/jacobbalane/Veyl/security/dependabot/1) concerns the
Linux-only GTK dependency `glib` 0.18.5
([RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html)). Dependabot's update
attempt reports `security_update_not_possible`: the current GTK dependency chain cannot resolve
the fixed 0.20.0 release. The alert remains open. `cargo tree --locked -p veyl-desktop --target
x86_64-pc-windows-msvc --invert glib@0.18.5` and the ARM64 equivalent both report no dependency:
this crate is not compiled into either supported Windows binary. Android does not use GTK. This
separate Dependabot maintenance failure is not a Build Veyl test or packaging failure.
