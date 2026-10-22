# Implementation progress

Last updated: 2026-10-22.

Section 0 followed: the full plan and the listed Glim references were read before writing source.
Work remains in milestone order. M0's local implementation is ready; the milestone is **not done**
until its GitHub, publishing and device acceptance checks pass. M1 has not started.

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
- [ ] **M0.9 [maintainer]** §10.7 steps 1–5: GitHub CLI, create and push the repo, turn on Pages,
      Android signing key and secrets, repository settings (homepage, topics, private vulnerability
      reporting, security alerts, labels).
- [ ] **M0.10 [maintainer] [device]** Pipeline dry run: add `## 0.0.1 (YYYY-MM-DD)` with "Preview
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
  The protocol-vector step remains commented until M1.3. Remote CI, attestations and deployment
  are not claimed before maintainer setup.

## Verification

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
| GitHub Build Veyl                                       | Passed: checks, Windows x64/ARM64, Android debug          |
| GitHub Pages workflow                                   | Passed; site deployed successfully                        |
| Live website browser checks                             | All three pages passed on desktop and mobile              |
| Downloaded CI artifacts                                 | Both installer checksums and debug APK signature verified |
| Release signing and attestations                        | Pending M0.9–M0.10                                        |

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
  verified locally; no release publication or phone installation is claimed).
- Website: `site/dist/`.

Build output and local checksums stay ignored. The x64 installer is about 2.18 MiB, ARM64 about
1.94 MiB, and the unsigned release APK below 1 MiB. These local artifacts have no GitHub provenance
attestation yet.

## Waiting on maintainer setup

**M0.9:** GitHub CLI is installed and authenticated as `jacobbalane`. After the maintainer continued
setup, the agent created and pushed [the public repository](https://github.com/jacobbalane/Veyl),
enabled Pages from Actions, and configured its homepage, topics, private vulnerability reporting,
Dependabot security alerts/fixes and the `device-report` label. The Website workflow built and
deployed successfully. Browser checks of the live home, docs and changelog pages passed at desktop
and mobile widths, including assets/fonts, download fallback and no page errors or overflow.

The first Build Veyl run exposed a Node 24 test-runner incompatibility with a directory argument.
CI and the documented command now use the explicit `scripts/*.test.mjs` glob. The corrected
[Build Veyl run](https://github.com/jacobbalane/Veyl/actions/runs/39704676134) passed Linux checks,
Windows x64 and ARM64 packaging, and Android tests/lint/debug packaging. Both downloaded Windows
installers match their checksums; the downloaded debug APK's signature verifies. These artifacts
remain in ignored `.cache/github/`, and have not been published as release downloads.

Workflow actions now use current stable release tags, checked against their upstream releases and
action inputs, and pass `actionlint`. This replaces the deprecated checkout, Node/Java setup,
Gradle, artifact, Pages and release action versions. The MSVC environment action's latest release
still declares Node 20; GitHub runs it with Node 24, and the Windows builds above passed.

The maintainer explicitly asked the agent to create the signing key. The permanent RSA 4096-bit
PKCS12 key is now outside the repository at `%USERPROFILE%\.veyl-signing\veyl-release.p12`. A random
256-bit password is encrypted for this Windows account in `veyl-release.password.dpapi`; directory
access is restricted to the current user and SYSTEM. All four GitHub signing secrets are
configured. No private key or password was printed, put in command arguments, or committed.

The local signed release APK builds and `apksigner verify` confirms the permanent certificate:
`a04d54b735f80adbf8c2549afb74883b019781e3340c57c5755ded7a0ac697c3` (SHA-256). The refreshed
[Build Veyl run](https://github.com/jacobbalane/Veyl/actions/runs/39705982601) also passed all checks,
both installers and the debug Android build before signing secrets were configured. CI now selects
the release APK when the configured secrets are available.

The first signed CI run found that AGP does not create `:app:testReleaseUnitTest` for this app's
current configuration. The release path now runs the existing debug app unit-test task and JVM
core tests, then release lint and signed packaging. The app task still has no test sources; the
two core tests supply the current Android logic coverage.

At the maintainer's request, all four signing values were exported to
`../VEYL_SIGNING_SECRETS.txt`, outside the Git repository, for saving in 1Password. This temporary
export is plaintext, restricted to the current user and SYSTEM, and must not be committed. Delete
it after the maintainer confirms the values are saved safely.

Still required for M0.9: the maintainer must save the actual password in a password manager and
make two independent backups of the key and password. Instructions are in
`%USERPROFILE%\.veyl-signing\BACKUP-INSTRUCTIONS.txt`; the DPAPI file alone is not a portable backup.
No tags or releases exist yet.

**M0.10:** Add the exact 0.0.1 preview changelog section, run the version script and checks, then
create/push the tag. Inspect the draft release for x64 and ARM64 installers, the signed APK, all
three checksums and attestations. Only the maintainer publishes the preview. Keep M0 incomplete
until the site downloads work and the device results below are recorded.

## Waiting on device tests

After successful CI and maintainer publication:

1. Download the Windows installer and Android APK from the website. Confirm the appropriate x64
   or ARM64 download, compare SHA-256 checksums and verify GitHub attestations.
2. On Windows, install and open Veyl. Confirm the title, tabs and version; actions stay unavailable.
   Close the window, reopen it from **Open Veyl** in the tray, launch a second instance, then use
   **Quit Veyl**. Report any failure.
3. On Android, install and open Veyl. Confirm Welcome, **Get started**, Home and version 0.0.1.
   Check light/dark themes, portrait/landscape and scrolling on the available phone.
4. Report Windows/Android versions, processor architecture, artifact versions and pass/fail for
   each step. The ARM64 installer needs an ARM64 PC for a real launch check.

No sensing or audio tests are appropriate for the 0.0.1 foundation. Those begin in later milestones.
