# Getting started

Veyl works two ways. You pick one when you first open it, and you can switch later in
**Settings → Sound source**.

- **With this PC:** your PC's speakers play and its microphone listens. No phone needed, and any
  app on the PC works, Spotify included. Laptops have everything built in.
- **With my phone:** your Android phone plays and listens. This works with any PC, including one
  without speakers.

These steps describe Veyl 0.1, the first release. Until it is out, the website offers a 0.0.1
preview that only opens the interface.

## What you need

- A PC with Windows 10 (version 1809 or later) or Windows 11, on x64 or ARM64.
- **With this PC:** speakers (built in, USB, wired or your monitor's, but not headphones) and a
  microphone near where your hand will move.
- **With my phone:** an Android phone or tablet with Android 8.0 or later, on the same home network
  as the PC. The PC may be wired to the same router. Sound must come from the phone's own speaker,
  not headphones or a Bluetooth speaker. Using sound from YouTube or YouTube Music needs Android 10
  or later; the Veyl player and the test sound work on every version.

## Install Veyl on your PC

1. Open [jacobbalane.github.io/Veyl](https://jacobbalane.github.io/Veyl/) and choose **Download
   for Windows**. The site picks the right file for your PC (x64 or ARM); the arrow next to the
   button lists the other downloads.
2. Run the installer. It installs Veyl for your user account only. The app isn't code-signed yet,
   so Windows SmartScreen may stop it: choose **More info**, then **Run anyway**.
3. Open Veyl. On **Welcome to Veyl**, choose **Get started**.
4. Veyl asks "How do you want to use Veyl?". Choose **With this PC** or **With my phone**. Veyl
   marks one as **Recommended** from what it finds: PC mode when your PC has speakers and a
   microphone and is a laptop, or has both built in. The choice is yours.

## With this PC

1. **Choose speakers and a microphone.** **Default device** follows Windows. Pick the speakers
   you will hear and the microphone closest to where you will poke.
2. Optionally, choose **Check this PC's sound**. Veyl plays a test sound for five seconds at your
   current volume, then says "This PC hears its own sound well." or suggests what to change.
3. **Play something.** Play a song or video on this PC. Any app works. Protected videos may not
   share their sound; music works best.

On a laptop, hold your hand about 20 cm in front of the screen. On a desktop, keep the microphone
within about 50 cm of where you poke. See [placement](placement.md).

## With my phone

1. **Get Veyl on your phone.** On your Android phone, open
   [jacobbalane.github.io/Veyl](https://jacobbalane.github.io/Veyl/) and tap **Download for
   Android**, then open the file. Android asks you to allow your browser to install apps. If Play
   Protect says it doesn't recognize the app, tap **More details**, then **Install anyway**.
2. **Open Veyl on the phone.** Tap **Get started** and **Allow microphone**. Notifications are
   optional but recommended. On Android 17 and later, also tap **Allow local network**, so your PC
   can connect.
3. **Pair your phone.** With Veyl open on the phone, the phone appears under **Nearby** on the PC.
   Choose **Pair**. Both screens show a 6-digit code. If the codes are the same, tap **Codes
   match** on both. You only pair once. If the phone doesn't appear, use **Add by address** (see
   [troubleshooting](troubleshooting.md)).
4. **Place your phone.** Lay it flat, screen up, in front of your monitor. Keep its speaker and
   microphones uncovered. Set the volume to about half or more. See [placement](placement.md).
5. **Play something, then tap Start.** Play a song or video on the phone; YouTube and YouTube
   Music work best. In Veyl on the phone, choose **Phone sound** and tap **Start listening**. The
   first start on a phone runs a sound check of about 10 seconds; keep the phone where it is.
6. **Allow Android's prompt.** The first time, Veyl explains the next step. Then Android asks to
   "record or cast"; allow it. Veyl only uses the sound your apps play, never your screen.
   Android asks again each time you start with Phone sound.
7. **Keep the phone's screen on.** Veyl stops listening when the screen turns off. The listening screen
   dims itself after 10 seconds; touch it to brighten it. If you switch to another app, Veyl
   shrinks to a small window that keeps the screen on.

Instead of Phone sound, you can choose **Veyl player** to play your own music files, or **Test
sound** for a bright test loop. Neither needs Android's prompt, and both work on Android 8 and 9.

If the phone is connected but not listening, **Ask phone to start** on the PC sends a "Tap to
start listening" notification to the phone. Android needs your tap to start.

## Try a poke

The **Live** tab on the PC shows the sound-quality meter and **What Veyl hears**, a picture of the
echoes that change. When you start, Veyl briefly shows "Finding the echo…" and "Learning the
room…". Keep your hand away until they pass.

1. Hold your hand about 20 cm above your phone, or about 20 cm in front of your microphone in PC
   mode.
2. Poke quickly toward it and back, as if pressing a button in the air.
3. Veyl says "Nice! Veyl saw that." in the first-run guide, and each poke appears under **Recent
   pokes**.

To practise, choose **Test pokes** and poke 10 times, about once a second. Veyl tells you how many
it saw and what might help. Actions never run during practice or the poke test. If the meter is
low or pokes are missed, see [the meter](meter.md) and [placement](placement.md).

## Turn on actions

Actions start off every time Veyl starts, so a stray poke can't click anything. Turn them on when
you're ready:

- the **Actions** switch at the top of the window,
- **Actions** in the tray menu, or
- **Ctrl+Alt+V**, from any app.

The same switch turns them off at once. When actions are on, each poke clicks where your pointer
is. In **Settings** you can change:

- **Poke action:** Click, Play or pause, Next track, Space bar or Nothing.
- **Sensitivity** (Low, Normal, High) and **Distance range** (Near, Normal, Far).
- **Actions hotkey:** choose **Change**, then press the new key combination.
- **Turn actions on when Veyl starts**, if you want them on from the start.
- **Start Veyl with Windows** and **Keep Veyl running in the tray when I close the window**.

To quit Veyl completely, choose **Quit Veyl** in the tray menu.

## Next

- [Placement](placement.md): where to put the phone or microphone, and what to avoid.
- [The meter](meter.md): what each bar means and how to raise it.
- [Troubleshooting](troubleshooting.md): every tip and message, with fixes.
- [Privacy](privacy.md): what Veyl captures, sends and stores.
