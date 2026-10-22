# Veyl

**Touch the air. Your screen follows.**

Veyl will turn the echo of the sound already playing on your Android phone or Windows PC into a
click when you poke the air. No camera or extra sensing hardware is needed.

**First release coming soon.** The M0 foundation includes the desktop and Android interfaces,
portable filter and test sound, website, and build workflows. Sensing, pairing and actions are
coming in v0.1. See [implementation progress](docs/progress.md) for verification and remaining steps.

**[Download from the website →](https://jacobbalane.github.io/Veyl/)** (planned) ·
[Windows and Android releases](https://github.com/jacobbalane/Veyl/releases) ·
[Docs](docs/getting-started.md) · [Changelog](CHANGELOG.md)

> Release installation notes: the Windows app will initially be unsigned. SmartScreen may offer
> **More info** → **Run anyway**. Android requires allowing your browser to install apps outside
> the Play Store; Play Protect may offer **More details** → **Install anyway**. Use the project's
> release files and compare their checksums. Checksums and build attestations are part of the
> planned release pipeline; no release has been verified yet.

## What it does

Coming in v0.1:

- Poke to click, play or pause media, skip a track, or press Space.
- Phone mode with Android 8 or later, or PC mode with speakers and a microphone.
- A live sound-quality meter and a view of moving echoes.
- A built-in test sound and a player for your own music files on Android.
- Actions start off. Ctrl+Alt+V will switch them on or off.
- Local, encrypted pairing and an explicit choice to save a test recording on your PC.

## Getting started

Coming in v0.1. You will need Windows 10 version 1809 or later, or Windows 11, on x64 or ARM64.

**With your PC:** install Veyl, choose **With this PC**, select speakers and a microphone, and play
a song or video. Headphones cannot provide the echoes Veyl needs.

**With your phone:** install both apps, put the phone on the same home network, pair by comparing
the code, and lay the phone flat in front of the monitor. Play sound through the phone's speaker
and start listening. Android 8–9 will use the Veyl player or test sound; copying another app's
sound requires Android 10 or later.

Then practise a poke about 20 cm from the phone or microphone. Turn actions on when ready.

## Which apps work

These are the plan's compatibility expectations, not completed Veyl device tests.

| Source                                   | Expected support                                                         |
| ---------------------------------------- | ------------------------------------------------------------------------ |
| YouTube and YouTube Music on Android 10+ | Playback capture allowed in the maintainer's recorder test on 2026-10-21 |
| Spotify on Android                       | Blocks playback capture; use the Veyl player or PC mode                  |
| Veyl player and test sound               | Coming in v0.1; work without playback-capture consent                    |
| PC apps, including Spotify               | Coming in v0.1 through Windows loopback; protected video may be silent   |
| Other Android apps                       | Depends on whether the app shares its sound                              |

## Privacy

Veyl's privacy requirements apply to every milestone. Sound stays on your devices. Phone audio
must be filtered before it leaves for a paired PC over the encrypted link. PC mode filters audio
immediately after capture. Nothing saves audio unless you start a test recording on the PC.
There are no accounts, analytics, telemetry or cloud services. See [privacy](docs/privacy.md).

## Try the interface

Run the desktop interface in a browser:

```sh
npm ci
npm run -w desktop dev
```

Open `http://127.0.0.1:1420`. The browser preview is labelled **Preview — sample data**.
The 0.0.1 foundation shows empty states and keeps sensing controls unavailable.

For the website, run `npm run -w site dev` and open `http://127.0.0.1:3000`.

## Build from source

See [development](docs/development.md) for prerequisites, commands and current build limits.
The Windows app uses Tauri, Rust and React. The Android app uses Kotlin and Compose.

## How it works

The digital copy of the playing sound gives Veyl a reference. Comparing that copy with the
microphone reveals echoes; changes in those echoes reveal movement. The planned meter will tell
you whether the current sound supports sensing. See [how it works](docs/how-it-works.md).

## Contributing

Veyl's code and brand assets are MIT licensed. Read [CONTRIBUTING](CONTRIBUTING.md),
[SECURITY](SECURITY.md) and [third-party notices](THIRD_PARTY_NOTICES.md).
Device reports are welcome. Never attach recordings of people talking, keys or secrets.
