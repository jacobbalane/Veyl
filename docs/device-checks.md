# v0.1 device checks

These checks need real phones and PCs. Passing tests, CI or the browser preview does not pass them.
They cover M5.4, M5.5, M6.6 and M7.5. Report each step as passed or failed, with the device, its
Windows or Android version and the build's commit. Never attach recordings of people talking.

## 1. Install the current build

The published 0.0.1 preview has no sensing. Use the artifacts of the newest successful
[Build Veyl](https://github.com/jacobbalane/Veyl/actions/workflows/build.yml) run on `main`:

- **Windows:** `Veyl-windows-x64` (or `Veyl-windows-arm64` on an ARM64 PC). Unzip it, compare the
  `.exe` with its `.sha256` file (`Get-FileHash -Algorithm SHA256 <file>`), then run it.
  SmartScreen may ask: **More info** → **Run anyway**.
- **Android:** `Veyl-android`. Allow your browser or file manager to install apps; Play Protect may
  ask: **More details** → **Install anyway**.

Builds from these artifacts are unreleased test builds. Releases stay the maintainer's step
(§10.8).

## 2. Test recordings and tuning (M5.4, gate G2)

On the PC, open Veyl, start listening, then **Record a test** on the Live tab: 60 s, with **Mark
pokes with the Space key** on. Press Space at the bottom of each poke while the Veyl window has
focus. Make recordings R1–R14 from the plan (§11.3):

| ID  | Setup                                                                               | Pokes |
| --- | ----------------------------------------------------------------------------------- | ----- |
| R1  | Bright pop song (YouTube Music), volume 50 %, hand 20 cm above the phone            | 10    |
| R2  | Same song, volume 70 %                                                              | 10    |
| R3  | Quiet piano or acoustic song, volume 60 %                                           | 10    |
| R4  | A YouTube video with talking, volume 60 %                                           | 10    |
| R5  | Veyl test sound, volume 50 %                                                        | 10    |
| R6  | Bright song, no pokes, typing and using the mouse for 2 minutes                     | 0     |
| R7  | Bright song, no pokes, someone walking behind the desk for 2 minutes                | 0     |
| R8  | Bright song, phone in its case                                                      | 10    |
| R9  | Bright song, phone turned so its bottom edge faces the monitor                      | 10    |
| R10 | Bright song, hand 12 cm and 30 cm high (5 pokes each)                               | 10    |
| R11 | PC mode on a laptop, bright song, hand 25 cm in front of the laptop                 | 10    |
| R12 | PC mode on a laptop, music playing, no pokes, typing for 2 minutes                  | 0     |
| R13 | PC mode with desktop speakers and a USB microphone (if available)                   | 10    |
| R14 | Phone mode with the Veyl player and an MP3 file, on a second phone of another maker | 10    |

R4 and R7 may record voices; keep them private. Recordings are in
`%LOCALAPPDATA%\io.github.jacobbalane.veyl\recordings\` (Help → saved recordings → **Open folder**). Copy each folder
into `lab/recordings/` in the repository (ignored by Git), named `R1`, `R2` and so on, then run:

```powershell
cargo run --release -p veyl-lab -- analyze lab/recordings/R1 --out .cache/lab/R1-report
```

**Gate G2:** on R1 and R2, the report's median `clarity_db` is at least 6 dB, and a hand at 20 cm
reaches `z_max` at least twice `z_on`. Share the reports (numbers only) for tuning; the tuning
results go into [tuning](tuning.md).

## 3. Processing cost and delay (M5.5)

On the target laptop (an older one, if available):

- With music playing and Veyl listening, open Task Manager → Processes and note the CPU column for
  Veyl over a minute, in phone mode and in PC mode.
- From the repository, run `cargo test --release -p veyl-sim --test performance -- --ignored`
  and report its output (60 s of audio should take under 2 s).
- After a poke test, note Help → Diagnostics → **Poke delay (median / p95)**.

## 4. Pokes and actions (M6.6, gate G3)

| Check | Steps                                                                                                                                      | Passes when                                   |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------- |
| D8    | Phone mode, bright song: **Test pokes** with the hand at 15 cm, then at 25 cm                                                              | At least 8 of 10 each                         |
| D9    | Phone mode, Settings → Poke action **Nothing**, actions on; 5 minutes of typing, mouse use and someone walking past; count the gesture log | At most 1 poke                                |
| D12   | Actions **Click**, then **Play or pause**; switch actions with Ctrl+Alt+V and the tray; restart Veyl                                       | Each works; actions are off after the restart |
| P3    | PC mode on a laptop: **Test pokes** with the hand 20–30 cm in front of it                                                                  | At least 8 of 10                              |
| P6    | PC mode, music playing, Poke action **Nothing**, actions on, 5 minutes of typing                                                           | At most 1 poke                                |
| Delay | Diagnostics after D8 and P3                                                                                                                | Median at most 200 ms, p95 at most 300 ms     |

Also try a click in an app run as administrator: Veyl should show "Windows blocked the click in an
app running as administrator."

**Gate G3:** recall at least 80 % and at most 1 false poke per 5 minutes.

## 5. The full checklist (M7.5)

Run D1–D17 and P1–P7 from the plan (§11.4), including the 60-minute battery and thermal check
(D11) and the ARM64 installer (P7) if an ARM64 PC is available. The phone-only capture checks are
in [phone device checks](phone-device-checks.md). Add each tested device to
[compatibility](compatibility.md).
