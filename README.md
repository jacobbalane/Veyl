# Veyl

**Touch the air. Your screen follows.**

Veyl turns the echo of the sound already playing on your Android phone or Windows PC into a click
when you poke the air. No camera or extra sensing hardware is needed.

**First release coming soon.** Version 0.1's features are built and tested in the simulator;
device testing is under way, and the published 0.0.1 preview only checks installation. See
[implementation progress](docs/progress.md) for what is verified and what remains.

**[Download from the website →](https://jacobbalane.github.io/Veyl/)** (planned) ·
[Windows and Android releases](https://github.com/jacobbalane/Veyl/releases) ·
[Docs](docs/getting-started.md) · [Changelog](CHANGELOG.md)

> Release installation notes: the Windows app will initially be unsigned. SmartScreen may offer
> **More info** → **Run anyway**. Android requires allowing your browser to install apps outside
> the Play Store; Play Protect may offer **More details** → **Install anyway**. Use the project's
> release files and compare their checksums. Checksums and build attestations are part of the
> planned release pipeline; no release has been verified yet.

## What it does

In v0.1:

- Poke to click, play or pause media, skip a track, or press Space.
- Phone mode with Android 8 or later, or PC mode with speakers and a microphone.
- A live sound-quality meter, a view of what Veyl hears, and a poke test.
- A built-in test sound and a player for your own music files on Android.
- Actions start off. Ctrl+Alt+V, the header switch or the tray switches them on or off.
- Local, encrypted pairing and an explicit choice to save a test recording on your PC.

## Getting started

You need Windows 10 version 1809 or later, or Windows 11, on x64 or ARM64.

**With your PC:** install Veyl, choose **With this PC**, select speakers and a microphone, and play
a song or video. Headphones cannot provide the echoes Veyl needs.

**With your phone:** install both apps, put the phone on the same home network, pair by comparing
the code, and lay the phone flat in front of the monitor. Play sound through the phone's speaker
and start listening. Android 8–9 will use the Veyl player or test sound; copying another app's
sound requires Android 10 or later.

Then practise a poke about 20 cm from the phone or microphone. Turn actions on when ready. See
[getting started](docs/getting-started.md) and [placement](docs/placement.md).

## Which apps work

These are the plan's compatibility expectations, not completed Veyl device tests.

| Source                                   | Expected support                                                         |
| ---------------------------------------- | ------------------------------------------------------------------------ |
| YouTube and YouTube Music on Android 10+ | Playback capture allowed in the maintainer's recorder test on 2026-10-21 |
| Spotify on Android                       | Blocks playback capture; use the Veyl player or PC mode                  |
| Veyl player and test sound               | Work without playback-capture consent                                    |
| PC apps, including Spotify               | Through Windows loopback; protected video may be silent                  |
| Other Android apps                       | Depends on whether the app shares its sound                              |

## Privacy

Sound stays on your devices. The phone removes the low sounds where voices are before anything
leaves it, and sends the rest only to your paired PC over an encrypted link. PC mode filters audio
right after capture and never sends it anywhere. Nothing saves audio unless you start a test
recording on the PC; recordings are deleted after 7 days by default and never uploaded. There are
no accounts, analytics, telemetry or cloud services. See [privacy](docs/privacy.md).

## Try the interface

Run the desktop interface in a browser:

```sh
npm ci
npm run -w desktop dev
```

Open `http://127.0.0.1:1420`. The browser preview is labelled **Preview — sample data**: it
connects to a sample phone and shows a hovering hand with a poke every 3 seconds. Address options
such as `?mode=pc` or `?tip=volume_low` pick a state; see [development](docs/development.md).

For the website, run `npm run -w site dev` and open `http://127.0.0.1:3000`.

## Build from source

See [development](docs/development.md) for prerequisites, commands and current build limits.
The Windows app uses Tauri, Rust and React. The Android app uses Kotlin and Compose.

## How it works

The digital copy of the playing sound gives Veyl a reference. Comparing that copy with the
microphone reveals echoes; changes in those echoes reveal movement, and a quick move toward the
microphone and back is a poke. The meter tells you whether the current sound supports sensing.
See [how it works](docs/how-it-works.md).

## Contributing

Veyl's code and brand assets are MIT licensed. Read [CONTRIBUTING](CONTRIBUTING.md),
[SECURITY](SECURITY.md) and [third-party notices](THIRD_PARTY_NOTICES.md).
Device reports are welcome. Never attach recordings of people talking, keys or secrets.
