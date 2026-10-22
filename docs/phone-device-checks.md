# Phone capture checks (M2)

These are real-device checks. A passing JVM test or CI build does not pass them. The published
0.0.1 preview has no capture Lab. Use the **debug APK built from the M2 source**, which installs
alongside the published app as **Veyl (debug)**. Never attach diagnostic recordings to an issue.

## Install the device build

Locally, build from `android/` with `./gradlew :core:test :app:testDebugUnitTest :app:lintDebug
:app:assembleDebug`. Install `android/app/build/outputs/apk/debug/app-debug.apk`, or download
`debug-android` from a successful [Build Veyl](https://github.com/jacobbalane/Veyl/actions) run
containing M2. The `Veyl-android` release artifact intentionally has no Lab.

With USB debugging enabled and a phone connected, install with:

```powershell
& "$env:LOCALAPPDATA\Android\Sdk\platform-tools\adb.exe" install -r android/app/build/outputs/apk/debug/app-debug.apk
```

## Gate G1: simultaneous capture

1. Open **Veyl (debug)**. Hold **Version 0.0.1** to open **Lab**. Read the device-check introduction.
2. In **Settings**, leave **Privacy filter: 8 kHz (recommended)** selected. Return to the Lab and
   select **Phone sound**. Use the phone's speaker. Disconnect headphones and Bluetooth audio. Open YouTube Music and
   play music with audible high sounds at about half volume or more. Some apps or tracks prevent
   Android playback capture; record which app and track you used if the reference stays silent.
3. Return to Veyl. Tap **Start capture check**, allow the microphone, then notifications if desired.
   Read **One more step**, tap **Continue**, and approve Android's recording/casting prompt.
   Veyl creates an audio capture session; it never creates a virtual display or captures images.
4. Keep both apps playing for **five minutes**, with Veyl visible or in picture-in-picture. YouTube
   Music without Premium may need to stay in front, with Veyl in picture-in-picture. Do not lock
   the phone. The debug Lab bypasses the ten-minute no-PC timeout for the later thirty-minute test;
   screen lock, projection loss, thermal and recorder errors still stop it.
5. Confirm both **Blocks** counts keep increasing (about 100 blocks per second per stream).
   **Reference → High sounds** should rise above its silence floor when the track has energy above
   8 kHz. **Microphone → Level** should respond to the music. Note any silence, errors or stalls.
6. Tap **Copy diagnostics** after five minutes, then **Stop**. Paste that metadata, the app's Git
   commit, pass/fail and any observed problem into the device result. It contains no samples or keys.

Optional: while running, tap **Record 10 seconds** and read the warning. Only **Record** starts
writing. These debug-only WAVs contain **unfiltered sound, including voices**. Each is capped at
ten seconds and stored only in this debug app's private `files/capture-lab/` folder. A stopped run
may leave a shorter, finalized recording. The app never shares or uploads them. Use **Delete
recordings** after checking, or clear the debug app's data/uninstall it. They are excluded from
Android backup. A new recording replaces the previous pair.

To inspect the optional WAVs locally, run the following from the repository root with a connected
phone. The Node helper preserves binary bytes; do not redirect `adb` WAV output in Windows PowerShell
5.1, whose text redirection corrupts binary files.

```powershell
node scripts/pull-lab-recordings.mjs
```

It copies only the two named files into ignored `lab/recordings/`. Nothing is uploaded. Delete
those local copies when finished. Compare reference/microphone spectra and listen locally to
confirm the microphone hears the music. Do not send the WAVs to the agent or commit them.

If G1 fails, record the phone/Android/app versions, source, rate, effects and exact failure. Apply
plan §14 R1: the Veyl player and test sound use a direct reference plus the microphone; PC mode
avoids phone playback capture. A device failure needs its own decision record before acceptance.

## Remaining M2 checks

The controls are implemented. Keep every result pending until exercised on a real phone:

- D5: connect/disconnect headphones or Bluetooth; the speaker warning follows the route and
  capture continues.
- D6: lock the phone; capture stops, its indicator clears and the alert explains the stop. Opening
  the app does not restart capture automatically. Restarting Phone sound needs fresh consent.
- D7: use picture-in-picture for five minutes. Confirm music, both counts and keep-screen-on;
  report the Android version and any phone-specific battery settings.
- D15: stop capture. In the Lab select **Veyl player**, then **Choose music** or **Choose a folder**.
  Pick local music and start. Exercise Play/Pause, Previous/Next and Shuffle with several tracks.
  Include a 44.1 kHz MP3; the reference and microphone must share the displayed run rate. Confirm no
  Android recording/casting dialog, and that the playlist survives closing/reopening the app.
  Gesture/poke checks wait for M6. Repeat with **Test sound**, which also needs no projection consent.
- D16: from Home, open **Settings → Run sound check**. Allow the microphone if needed. Confirm music
  plays at the media volume, the check completes in about ten seconds, and its result survives a
  restart. Repeat the check. The Lab uses the saved choice on the next run; it deliberately skips
  the automatic first-run check so G1 can also exercise the default recorder fallback. Production
  listening runs the check automatically when this Android system version has no saved result.
- D17: if available, use an Android 8/9 phone with test sound/player; Phone sound must explain that
  it needs Android 10.
- Leave YouTube Music and the Lab running for thirty minutes; report errors, dropped/stalled
  counts, route, phone temperature warnings and whether music stayed audible.
- While Phone sound is running, switch to Test sound and back. Confirm the projection stays live,
  returning to Phone sound needs no second dialog and both counts restart for the new run.
- Confirm dimming after ten seconds, touch to restore brightness, dim/PiP settings, the optional
  battery settings link, About/Licenses, microphone refusal with Open settings, cancelled projection
  consent, notification Stop, and Stop before restarting. None may start recording automatically.

## Result to record

```text
Source commit / APK:
Phone / Android version:
Music app / track:
G1, 5 min (pass/fail + diagnostics):
D5 / D6 / D7:
D15 / D16 / D17 (or unavailable hardware):
30 min (pass/fail + diagnostics):
Issues and fallback applied:
```
