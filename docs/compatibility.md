# Compatibility

Veyl is meant to work on any Android phone and any Windows PC that meets the baseline, whatever
the brand or model. It checks what each device can do while it runs and falls back when something
is missing. This page records which devices have actually been tested, and how.

## Baseline

|                    | Needed                                                        | For all features and the best results                                                 |
| ------------------ | ------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| Phone              | Android 8.0 or later, any brand, phone or tablet              | Android 10 or later to use the sound of other apps, such as YouTube and YouTube Music |
| PC                 | Windows 10 version 1809 or later, or Windows 11; x64 or ARM64 | The same                                                                              |
| Phone mode network | Phone and PC on the same local network                        | The same                                                                              |
| PC mode            | Speakers (not headphones) and a microphone                    | Built-in or wired speakers, and a microphone that offers a raw mode                   |

When something is missing, Veyl falls back: the Veyl player and the test sound on phones that
can't share other apps' sound, the next microphone setup when the best one isn't available, mono
when there is no stereo microphone, ordinary capture when a PC microphone has no raw mode, and
**Add by address** when the network blocks discovery. When nothing works, Veyl says so and
suggests the other mode.

## How to read the tables

The maintainer fills in these tables from their own devices and from device reports. A row stays
**Not yet tested** until someone has run that check with Veyl on that device. Results from other
apps or from the simulator don't count.

- **Sound check:** the phone's choice of microphone setup and its score, or the PC sound check's
  result.
- **Poke test:** how many of 10 pokes **Test pokes** caught, once with the test sound and once with
  a song.

## Phones

| Model                             | Android     | Veyl           | Sound check    | Stereo or mono | Sample rate    | Apps that share sound | Poke test (test sound / song) | Notes                                                                                                                                                        |
| --------------------------------- | ----------- | -------------- | -------------- | -------------- | -------------- | --------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Maintainer's phone (model to add) | 10 or later | Not yet tested | Not yet tested | Not yet tested | Not yet tested | Not yet tested        | Not yet tested                | Before Veyl existed, a third-party screen recorder on this phone could copy YouTube and YouTube Music but not Spotify (2026-10-21). That is not a Veyl test. |

## PCs

| Model or type           | Windows and architecture | Speakers and microphone      | Raw mode       | Bluetooth      | PC sound check | Poke test      | Notes                                                            |
| ----------------------- | ------------------------ | ---------------------------- | -------------- | -------------- | -------------- | -------------- | ---------------------------------------------------------------- |
| Maintainer's desktop PC | Windows 11 Pro           | USB microphones, no speakers | Not yet tested | Not yet tested | Not yet tested | Not yet tested | Phone mode only, unless speakers are added. Wired to the router. |

No Veyl device checks have been completed yet. The planned checks for v0.1 cover at least three
phones from different makers (one with Android 8 or 9 if one can be found) and two PCs, one of
them a laptop.

## Add your device

Device reports are welcome, and each one can become a row here.

1. Use Veyl for a while and run **Test pokes**, once with the test sound and once with a song.
2. Copy the diagnostics: on the phone, **Settings → Copy diagnostics**; on the PC, **Help →
   Diagnostics → Copy diagnostics**. They contain no sound, codes or keys, but they do contain
   device names and models.
3. Open a [Device report](https://github.com/jacobbalane/Veyl/issues/new/choose) on GitHub and
   fill it in.

Never attach recordings of people talking, pairing codes or keys.
