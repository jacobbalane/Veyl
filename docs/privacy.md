# Privacy

Veyl listens to sound, so it is built to keep that sound at home. It removes the low sounds where
voices are before anything else sees them, sends sound only to your own paired PC, saves nothing
unless you start a test recording, and has no accounts, analytics or cloud services.

## The rules Veyl follows

These rules are requirements for every part of Veyl:

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

## What Veyl captures

**With your phone.** Veyl captures two things, and only after you tap **Start listening**:

- The microphone, which hears the echoes and everything else around the phone.
- A copy of the sound being played. With **Phone sound** (Android 10 or later), Android asks you
  to allow "record or cast" each time you start. That is how Android lets apps share their sound;
  Veyl uses only the sound, never your screen. The **Veyl player** and the **Test sound** need no
  such prompt, because Veyl plays that sound itself.

While it listens, the phone shows the listening screen, a "Veyl is listening" notification with
the connected PC's name, and Android's own screen-capture or microphone indicator where Android
provides one. Tap **Stop** to end it. Listening
also ends when the screen turns off.

**With this PC.** Veyl captures Windows' copy of the sound sent to the speakers you chose, and the
microphone you chose. It does this whenever Veyl runs in PC mode, including when its window is
closed to the tray. To stop it, choose **Quit Veyl** in the tray menu or switch to phone mode in
Settings. Windows also shows when an app is using the microphone.

Veyl never uses a camera.

## The privacy filter

Before sound goes anywhere else, Veyl removes everything below 8 kHz with a steep (eighth-order)
high-pass filter. Most of what makes speech understandable lies below 4 kHz; the filter lowers
those sounds by more than 40 dB, and automated tests check that on both the phone and the PC.
Veyl senses only with the high sounds that remain. Both streams pass the same filter, so the
echoes Veyl measures in that range stay the same.

The filter makes speech very hard to understand. It does not make it impossible: some high parts
of speech, such as "s" sounds, pass through.

- **On the phone**, the filter runs before anything is sent. Settings offers **8 kHz
  (recommended)**, **7 kHz** and **6 kHz**. Choosing a lower setting asks you to confirm and
  shows this warning: "Lower settings send more of the sound around you to your PC, including more
  of people's voices. Veyl still never saves or uploads it."
- **On the PC**, in PC mode, the 8 kHz filter runs right after capture, before the echo engine, a
  test recording or the sound check sees the sound. There is no lower setting.

Each 10 ms block also carries two numbers measured before the filter: how loud it was overall and
its highest peak. The meter uses them to tell a quiet song from a blocked one.

## What is sent, and where

**With your phone**, sound goes only to a PC you paired, and only over Veyl Link, Veyl's
encrypted connection on your local network:

- Pairing happens once. Both screens show the same 6-digit code, and you confirm on both.
- Every connection uses fresh keys. The phone accepts one PC at a time and always shows which one
  is connected. You can forget a PC on the phone (Settings → Paired PCs) or a phone on the PC
  (Phones tab).
- The phone sends the filtered sound and its timing; whether it is listening and with which
  source; its maker, model, Android version, Veyl version and name; its battery level, charging
  state, temperature status, processor use and media volume; and details of the microphones it
  uses.
- The PC sends back its name and Veyl version, the meter's score and tip, each poke, and a
  request to start when you click **Ask phone to start**.

**With this PC**, sound never leaves the PC. Veyl doesn't look for phones or use the network at
all in PC mode.

## What is stored

**On the PC**, in `%LOCALAPPDATA%\io.github.jacobbalane.veyl\`:

- Settings and the list of paired phones. Each phone's link key is protected by Windows for your
  user account.
- Logs, one file per day, the last seven days. They never contain sound, keys, nonces or pairing
  codes.
- Test recordings, only when you choose **Record a test** and confirm. A recording holds the
  filtered sound of both streams for 30, 60 or 120 seconds, its timing, the moments you pressed
  Space and the names of the devices used. It stays on this PC. Veyl deletes recordings older
  than 7 days each time it starts. With **Keep test recordings for 7 days** turned off in
  Settings, they stay until you delete them. You can delete one any time from the list of test
  recordings.

Uninstalling with "delete app data" removes this folder.

**On the phone**: settings (including the phone name and filter setting), the sound check's
choice of microphone, the music you chose for the Veyl player, and the list of paired PCs. Link
keys are wrapped by a key that stays inside Android's Keystore. Veyl's data is excluded from cloud
backup and device transfer. The phone app never saves sound. (Development builds made for device
checks have a hidden Lab that can save 10 seconds of unfiltered sound after a warning. The app on
the website doesn't include it.)

**Copy diagnostics**, on either device, copies text to your clipboard: versions, device models and
names, audio settings and recent states. It never includes sound, pairing codes or keys. You
decide where to paste it; a GitHub issue is public.

## What is never sent

- Nothing goes to the internet. There are no accounts, analytics, telemetry, crash reports, ads or
  cloud services.
- Test recordings are never uploaded. If you share one to report a problem, that is your choice;
  never share recordings of people talking.

## Network traffic

This is all of it:

- **Veyl Link**, between your phone and your PC on the local network, in phone mode only. The
  phone listens on port 47820 (or another free port if that one is taken); the PC connects out to
  it, so Windows never needs to accept a connection. Everything after the first handshake is
  encrypted.
- **DNS-SD announcements.** While Veyl is open or listening, the phone announces itself as a Veyl
  phone on the local network, with its name, model and a short ID, so your PC can find it. Anyone
  on the same network can see that announcement. You can change the name in the phone's Settings
  (**Phone name**). In phone mode, the PC asks Windows to look for these announcements.
- **GitHub**, only when you open a link, such as the website or the source code.

Checking for updates arrives in v0.2. The Windows installer may also download Microsoft's WebView2
runtime if your PC doesn't have it yet.

For the details of the encryption and pairing, see [the protocol](protocol.md) and Veyl's
[security policy](https://github.com/jacobbalane/Veyl/blob/main/SECURITY.md).
