# If something isn't working

The published 0.0.1 preview opens the interface and cannot pair or listen. The M2 development build
adds phone capture and a debug Lab. Pairing and streaming arrive in M3. Check
[progress](progress.md) for the exact build and limits.

## Music is playing, but the reference is silent

Some Android apps prevent other apps from capturing their sound. Try YouTube or YouTube Music
through the phone's speaker. If the app still blocks capture, choose **Veyl player** and pick your
own music, or **Test sound**. Both use their exact playback as the reference and need no Android
recording/casting consent. Microphone permission is still needed. **Phone sound** requires Android
10 or later; the own-player sources also work on Android 8 and 9.

**Run sound check** in Settings chooses a microphone setup for this phone and Android system
version. If it cannot hear high sounds, try a higher media volume, remove a case covering the
speaker/microphones, or later use PC mode. Changing the privacy filter below the recommended 8 kHz
requires a deliberate choice and warning; it sends more of the sound around you to your PC.

## Veyl stops when the phone locks

Keep the phone screen on while listening. Veyl dims its own screen after ten seconds; touch it to
restore normal brightness. The setting can turn dimming off. Veyl enters picture-in-picture when
you switch apps if the phone supports and permits it. On some phones, picture-in-picture may not
keep the screen awake: keep Veyl open or increase the system screen timeout.

YouTube Music without Premium may stop when it leaves the foreground. Keep YouTube Music in
front with Veyl in picture-in-picture. Confirm this on your phone with the
[device checks](phone-device-checks.md); JVM tests cannot prove it.

Locking the phone, ending Android's capture consent, a failed recorder, revoked microphone access,
critical temperature or ten minutes without a PC stops listening. It never resumes by itself.
Open the app and start again; Phone sound needs fresh consent after the previous projection ends.
The explicit debug Lab bypasses only the no-PC deadline so the thirty-minute check can finish.

## Headphones, Bluetooth or another app

Veyl needs the phone's own speaker. Unplug headphones or change the Bluetooth route when the
speaker warning appears. Capture continues while you fix the route. If another app or a call
takes the microphone, Veyl shows its warning until microphone sound returns. Pause other sounds
while using the test loop.

Some phones stop apps in the background. **Settings → Let Veyl run without limits** opens Android's
battery settings only when you choose it. The phone decides whether to grant the exception; it
does not make screen-lock capture work.

## Diagnostics and local music

**Settings → Copy diagnostics** copies the app/Android version, phone model, microphone setup,
sample rate, route, privacy cutoff and twenty recent state changes. It contains no audio, pairing
codes or keys. The debug Lab adds live counts and levels. Include the source commit and any exact
failure when reporting a problem. Never upload Lab recordings.

Choose music with Android's document picker or choose a folder. Veyl remembers the document URIs
without broad storage access. Moving a file or removing a provider's access can make a saved item
unavailable; choose the music again. Mono is duplicated to stereo and other channel layouts are
downmixed. A 44.1 kHz file is resampled to match the microphone's run rate.

## A PC cannot find the phone

Connection is still being implemented in M3. It will use the local network on both devices. Guest
networks and client isolation may block discovery; Add by address is the planned fallback. The
0.0.1 preview and M2 null-link Lab do not establish a PC connection.
