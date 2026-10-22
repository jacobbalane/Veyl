# Privacy

These are requirements for every Veyl implementation. The foundation does not capture audio.

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

The filter makes speech very hard to understand, not impossible. DNS-SD discovery exposes the
phone's name and model on the local network. Pairing keys and settings remain on each device.
