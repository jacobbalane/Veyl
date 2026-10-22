# Release and maintainer setup

M0 is in progress. These commands are for the maintainer after the local foundation checks pass.
They have not been run. Repository setup enables CI; wait for green CI before tagging a preview.
Create the signing key outside the repository; keep its password private and retain two backups.

## One-time setup

1. Install the GitHub CLI and log in: `winget install --id GitHub.cli -e`, then `gh auth login`.
2. After Codex's first commits, in `projects\Veyl`:
   `gh repo create jacobbalane/Veyl --public --source . --remote origin --push --description "Touch the air. Your screen follows. Control your Windows PC with a poke in the air, using your Android phone as a sonar."`
3. Turn on Pages from Actions: `gh api -X POST repos/jacobbalane/Veyl/pages -f build_type=workflow`
   (or Settings → Pages → Source: GitHub Actions).
4. Create the Android signing key **once and keep it forever** (losing it means people must uninstall
   Veyl to install an update):

   ```powershell
   $veylSigningDirectory = Join-Path $env:USERPROFILE '.veyl-signing'
   New-Item -ItemType Directory -Force -Path $veylSigningDirectory | Out-Null
   $veylSigningKey = Join-Path $veylSigningDirectory 'veyl-release.p12'
   keytool -genkeypair -v -storetype PKCS12 -keystore $veylSigningKey -alias veyl -keyalg RSA -keysize 4096 -validity 10000 -dname "CN=Veyl, O=Veyl contributors"
   [Convert]::ToBase64String([IO.File]::ReadAllBytes($veylSigningKey)) | gh secret set VEYL_KEYSTORE_BASE64 --repo jacobbalane/Veyl
   gh secret set VEYL_KEYSTORE_PASSWORD --repo jacobbalane/Veyl
   gh secret set VEYL_KEY_ALIAS --repo jacobbalane/Veyl --body veyl
   gh secret set VEYL_KEY_PASSWORD --repo jacobbalane/Veyl    # same as the keystore password for PKCS12
   ```

   Keep `veyl-release.p12` and its password in a password manager and one more safe place. Never in
   the repository.

5. Repository settings:

   ```powershell
   # Website link and topics shown on the repository page.
   gh repo edit jacobbalane/Veyl --homepage "https://jacobbalane.github.io/Veyl/" --add-topic sonar,gesture-control,touchless,accessibility,android,windows,tauri,rust,kotlin,open-source
   # SECURITY.md points people to private vulnerability reporting, so it must be on.
   gh api -X PUT repos/jacobbalane/Veyl/private-vulnerability-reporting
   # Dependabot security alerts and automatic security fixes.
   gh api -X PUT repos/jacobbalane/Veyl/vulnerability-alerts
   gh api -X PUT repos/jacobbalane/Veyl/automated-security-fixes
   # Labels used by the issue templates.
   gh label create device-report --repo jacobbalane/Veyl --color 4FD8C4 --description "A report about how Veyl works on a phone or PC"
   ```

6. Optional: protect `main` so the `Build Veyl` checks must pass (Settings → Rules → Rulesets, or
   `gh api` with a ruleset). Optional: a tag ruleset so only the maintainer can create `v*` tags.

**Checking the GitHub side works** (after M0.10): the Actions tab shows green runs of "Build Veyl"
and "Website"; Settings → Pages shows the site URL; the Releases page lists v0.0.1 with six files
(the x64 and ARM64 installers, the APK, and their checksums) and their attestations (`gh attestation verify Veyl_0.0.1.apk --repo jacobbalane/Veyl` succeeds);
the website's buttons download those files.

## The 0.0.1 pipeline check

After main passes CI, add this section to CHANGELOG.md:

```markdown
## 0.0.1 (YYYY-MM-DD)

Preview build. Nothing works yet; this release tests downloads.
```

Use the actual release date. Keep later feature notes under Unreleased. Run
`node scripts/bump-version.mjs 0.0.1`, `npm run format:check`, `node --test scripts/` and
`node scripts/check-version.mjs`, then commit `chore(release): 0.0.1` and push main. After its checks
pass, the maintainer creates and pushes `v0.0.1` as below.

Check the draft's six download files and attestations. Publish it as
**0.0.1 — preview build (no features yet)**, then perform the installation-only checks in
[progress.md](progress.md). Sensing, pairing and gesture checks apply to v0.1, not this foundation.

## Releasing v0.1 and later

1. Move the **Unreleased** entries in `CHANGELOG.md` under `## X.Y.Z (YYYY-MM-DD)` with a one-line
   summary.
2. `node scripts/bump-version.mjs X.Y.Z` and commit `chore(release): X.Y.Z`.
3. **[maintainer]** `git tag vX.Y.Z` and `git push origin vX.Y.Z`.
4. CI creates a **draft** release named "Veyl vX.Y.Z" with `Veyl_X.Y.Z_x64-setup.exe`,
   `Veyl_X.Y.Z_arm64-setup.exe`, `Veyl_X.Y.Z.apk`, their `.sha256` files and build attestations. Its notes are the version's
   CHANGELOG section followed by GitHub's generated list of changes. Watch the run with
   `gh run watch` or in the Actions tab; if a job fails, fix it on `main`, delete the tag
   (`git push --delete origin vX.Y.Z`, `git tag -d vX.Y.Z`) and tag again.
5. **[maintainer]** Install both from the draft and run the release checklist: fresh install on PC
   and phone · pair · YouTube Music: meter and echo view respond · poke test ≥ 8/10 · actions switch
   and hotkey · Spotify shows the blocked tip · Bluetooth headphones show the speaker tip · locking
   the phone shows the stop message · PC mode on a laptop: meter, poke test ≥ 8/10, Spotify works ·
   uninstall both cleanly. Then publish. The website's buttons
   point to the new files immediately (they read the latest release).
