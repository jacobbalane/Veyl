# If something isn't working

Find the words Veyl shows you, then follow the fix below. Each section starts with Veyl's exact
text. The meter shows one tip at a time, the most important first, so after one fix another tip
may appear.

The sections are: tips from the meter, status messages on the PC, errors on the PC, messages on
the phone, missed pokes, finding your phone on the network, phones that stop Veyl in the
background, and reporting a problem.

## Tips from the meter

Where the phone and the PC use different words, both are shown. See [the meter](meter.md) for
what each bar measures.

### Veyl isn't listening

- With your phone: "Tap Start in Veyl on your phone."
- With this PC: "Veyl isn't listening. Choose your speakers and microphone in Settings."

With your phone, open Veyl on the phone and tap **Start listening**. If the phone is connected,
**Ask phone to start** on the PC sends it a "Tap to start listening" notification. If listening
stopped by itself, the phone says why; see **Messages on the phone** below.

With this PC, open **Settings**, check that **Sound source** is **With this PC**, and choose
**Speakers** and **Microphone**. **Default device** follows Windows.

### Sound goes to headphones or Bluetooth (phone)

> Sound is going to headphones or Bluetooth. Veyl needs your phone's speaker.

Veyl hears echoes of the sound that comes out of the phone's own speaker. Unplug headphones, or
disconnect Bluetooth headphones and speakers, or switch the sound back to the phone in Android's
media output controls. Veyl keeps listening and carries on as soon as the sound returns to the
speaker. The phone shows the same words as a banner.

### Sound goes to headphones (PC)

> Your PC's sound is going to headphones. Veyl needs speakers.

The speakers Veyl uses are headphones or a headset. Unplug them, or choose other speakers in
**Settings → Speakers**. With **Default device**, Veyl follows Windows when it switches back to
the speakers. If your PC has no speakers, use Veyl with your phone instead.

### No microphone or speakers chosen (PC)

> Choose a microphone in Settings.

> Choose speakers in Settings.

Veyl has no microphone or no speakers to use. Choose them in **Settings**. If a device shows as
"Unavailable device", it isn't connected; plug it back in, or choose another device or **Default
device**. Veyl keeps a device you chose rather than quietly switching to another one.

### The app doesn't share its sound

- With your phone: "This app doesn't share its sound with Veyl. Try YouTube, YouTube Music, the
  Veyl player or the test sound."
- With this PC: "Veyl can't copy this sound. Protected videos don't share it. Try music or another
  video."

Some Android apps, such as Spotify, block other apps from copying their sound. Use YouTube or
YouTube Music, choose **Veyl player** to play your own music files, or choose **Test sound**. Or
use Veyl with this PC: on a PC, every app shares its sound, Spotify included.

On a PC, protected (DRM) video can reach Veyl as silence. Play music or another video.

### Nothing is playing

- With your phone: "Play some music or a video on your phone."
- With this PC: "Play some music or a video on this PC."

Start something, and check that it isn't muted or paused. In PC mode, the sound must play through
the speakers Veyl uses: if an app plays to other speakers, Veyl's copy stays silent. On the PC,
the status may also say "This PC isn't hearing any sound".

### Another app is using the microphone

- With your phone: "Another app is using your phone's microphone."
- With this PC: "Another app is using the microphone. Close it, or choose another microphone."

A call, a voice assistant or a recording app has taken the microphone. On the phone, Veyl keeps
running and continues when the microphone is free again. On the PC, close the other app (some
apps take a microphone for themselves), or choose another microphone in **Settings**. Also check
that Windows lets desktop apps use the microphone, in Windows Settings under **Privacy &
security → Microphone**.

### The sound is distorting

> Turn the volume down a little. The sound is distorting.

The microphone is overloaded. Lower the volume a step at a time until the tip changes. In PC mode,
if the speakers are already quiet, lower the microphone's input level in Windows' sound settings,
or move the microphone a little farther from the speakers.

### The volume is too low

- With your phone: "Turn your phone's volume up."
- With this PC: "Turn your PC's volume up."

Raise the media volume; on a phone, about half or more is a good start. Keep the microphone
uncovered. In PC mode, move the microphone closer to where you poke.

### Few high sounds

> This track has few high sounds. A brighter song works better.

Veyl senses with sounds above 8 kHz: cymbals, hi-hats, "s" sounds. Soft piano, bass-heavy music
and talk have few of them. Try a brighter song, or the test sound. If your music app has a
streaming quality setting, a higher one can help.

### Many quiet moments

> This track has many quiet moments.

Veyl can't see your hand while nothing is playing. Choose music without long pauses, or a
playlist.

### Other sounds during the test sound

- With your phone: "Pause other sounds while the test sound plays."
- With this PC: "Pause other sounds while the sound check runs."

Another app is playing at the same time. Its sound isn't part of the copy Veyl compares with, so
it blurs the echo. Pause it.

### The echo is unclear

- With your phone: "The echo is unclear. Keep the phone's speaker and microphones uncovered."
- With this PC: "The echo is unclear. Move the microphone closer to where you poke."

The copy of the sound no longer explains what the microphone hears. To make it clearer:

- Take off a case that covers the speaker or microphones, and move papers or a hand resting on
  the phone.
- Pause other sound near the microphone: a TV, a second speaker, people talking.
- Turn off sound enhancers such as "Dolby Atmos", virtual surround or equalizer effects in the
  phone's sound settings. On a PC, turn off audio enhancements for the speakers in Windows' sound
  settings. These change the sound on its way to the speaker, so it no longer matches the copy.
- In PC mode, move the microphone closer to where you poke. See [placement](placement.md).

### Sudden noises

- With your phone: "Sudden noises near the phone make the echo hard to read."
- With this PC: "Sudden noises near the microphone make the echo hard to read."

Typing, tapping the desk or knocks near the microphone are loud and short. Veyl leaves out short
bursts, but frequent ones get in the way. Move the phone or microphone away from the keyboard, or
put something soft under a noisy keyboard.

### Bluetooth speakers (PC)

> Bluetooth speakers add delay. Built-in or wired speakers work better.

Veyl works with Bluetooth speakers, but their delay changes and makes the echo harder to follow.
Choose built-in or wired speakers in **Settings → Speakers** if you have them.

### Ready

- With your phone: "Ready. Poke toward your phone."
- With this PC: "Ready. Poke toward your microphone."

The sound is good. If pokes are still missed, see **Pokes are missed, or nothing happens**
below.

## Status messages on the PC

### Looking for your phone

> Looking for your phone…

Open Veyl on the phone. The phone announces itself only while Veyl is open or listening. Both
devices must be on the same network. If the phone still doesn't appear, see **The PC can't find
your phone** below.

### Phone found

> Phone found · pair it to start

Open the **Phones** tab and choose **Pair** next to your phone, then compare the codes.

### Connected, not listening

> {phone} connected · tap Start on your phone

Tap **Start listening** in Veyl on the phone, or choose **Ask phone to start** on the PC and tap
the notification on the phone.

### Reconnecting or can't reach

> Reconnecting to {phone}…

> Can't reach your phone

The connection dropped, for example because the phone's Wi-Fi changed, Veyl was closed on the
phone, or the phone left the network. Veyl keeps trying; after about 30 seconds it says it can't
reach the phone, and still keeps trying. Open Veyl on the phone and check that it is on the same
network as the PC. On Android 17 and later, check that Veyl still has local network access.

### No sound in PC mode

> Listening through this PC · This PC isn't hearing any sound

Veyl is capturing, but the speakers it uses are silent. See **Nothing is playing** above.

### Starting up

> Waiting for sound…

> Finding the echo…

> Learning the room…

These are normal for a few seconds after you start. Veyl first waits for sound, then lines up the
copy with the microphone, then learns the still parts of the room. Keep your hand away and the
phone still until they pass. If Veyl stays on one of them, follow the meter's tip: usually nothing
is playing, the app doesn't share its sound, or the sound has few high parts.

### Sound too weak

> Sound too weak to detect pokes

The score is very low, so Veyl has stopped looking for pokes to avoid stray clicks. Follow the
meter's tip; detection resumes when the sound improves.

## Errors on the PC

### Couldn't verify your phone

> Couldn't verify your phone. If you reinstalled Veyl on it, pair again.

The phone no longer recognizes this PC. That happens after reinstalling Veyl on the phone,
clearing its data, or forgetting this PC on the phone. Forget the phone in the **Phones** tab,
then pair again. If you connected by address to a phone you haven't paired yet, choose **Pair**.
Veyl never deletes a pairing by itself.

### Different versions

> Update Veyl on your phone and PC so both have the same version.

Install the latest Veyl from [the website](https://jacobbalane.github.io/Veyl/) on both
devices.

### The phone locked

> Your phone stopped listening because its screen locked. Open Veyl on your phone and tap Start.

Veyl on the phone stops listening when its screen turns off. Open Veyl on the phone, tap **Start
listening**, and keep the screen on; see **Veyl stops when the phone locks** below.

### Windows blocked the click

> Windows blocked the click in an app running as administrator.

Windows doesn't let ordinary apps send clicks or keys to an app running as administrator. Veyl
noticed and didn't send it. Use pokes with other windows, or run that app without administrator
rights if you can.

### Pairing messages

> Open Veyl on your phone, then try again.

Pairing works only while Veyl is open in front on the phone. Open it, then choose **Pair** again.

> Too many tries. Wait a few minutes, then try again.

The phone allows at most 10 pairing attempts in 10 minutes, then pauses pairing for 10 minutes.

> Pairing didn't finish. Try again.

Someone tapped **Cancel**, the codes weren't confirmed within two minutes, or a check failed.
Try again. If the two codes ever differ, tap **Cancel** on both: something else may be answering
on your network.

> Your phone is connected to another PC.

The phone talks to one PC at a time. Disconnect the other PC (**Disconnect**, or quit Veyl
there), then try again.

### Address errors

> Enter the address shown on your phone, including its port.

Type the address exactly as the phone's home screen shows it after "Address:", for example
`192.168.1.23:47820`.

### Connection requests

> Veyl couldn't complete that request. Try again.

> Veyl's connection manager is busy. Try again.

Wait a moment and try again. If it keeps happening, quit Veyl from the tray and open it again.

### Paired devices

> Veyl couldn't save paired devices. Try again.

> Veyl couldn't load paired devices. Try again.

Veyl couldn't read or write its list of paired devices. These messages can appear on the phone
or the PC. Check that the device has free storage, then restart Veyl. If loading keeps failing,
pair again.

### PC audio devices

> Your audio device changed. Veyl is reconnecting it.

Windows switched the default device, or a device was unplugged or changed. Veyl restarts capture
by itself. If it doesn't recover, choose the devices again in **Settings**.

> Veyl couldn't open these audio devices. Check them in Settings.

Windows refused to open the speakers or microphone. Another app may be using one for itself, the
device may be disabled in Windows' sound settings, or Windows may not let desktop apps use the
microphone (**Privacy & security → Microphone**). Veyl tries again every few seconds. Choose
other devices if it keeps failing.

> Veyl couldn't save your sound settings. Try again.

Veyl couldn't save your choice of sound source or devices. Try again; if it keeps failing, check
that your PC has free disk space.

### PC sound check

> This PC hears its sound poorly. Move the microphone closer to where you poke, or turn the volume up.

Very little of the test sound reached the microphone. Move the microphone closer, turn the
volume up, use speakers rather than headphones, and turn off audio enhancements. Then run
**Check this PC's sound** again. If the PC can't hear itself well, Veyl with your phone may work
better.

> The sound check didn't finish. Check your devices and try again.

The check was cancelled, a device changed, or the sound was interrupted. It needs Veyl listening
with this PC and both devices working. Try again.

### Test recordings

> Start listening before recording a test.

A test recording saves what Veyl hears, so it needs sound. Start listening first.

> Recording didn't finish. You can open or delete the partial files.

Listening stopped, a device or the connection changed, or the PC couldn't write fast enough. The
part that was saved is kept; open or delete it from the list of test recordings.

> Veyl couldn't save this test. Try again.

> Veyl couldn't load test recordings. Try again.

> Veyl couldn't delete this recording. Try again.

> Veyl couldn't open the recordings folder.

Check that your PC has free disk space, and close any app that has a recording's files open.
Then try again.

### Settings, window and diagnostics

> Veyl could not load its settings.

> Veyl could not open its window.

> Veyl could not copy diagnostics.

Quit Veyl from the tray and open it again. If it keeps happening, please report it with your
Windows version; see **Diagnostics and reporting a problem** below.

## Messages on the phone

### Microphone permission

> Veyl needs the microphone to hear echoes.

Tap **Open settings**, then allow the microphone for Veyl in Android's app settings. Veyl removes
the low sounds where voices are before anything leaves the phone; see [privacy](privacy.md).

### Android's prompt was cancelled

> Veyl needs this to hear what your phone plays.

With **Phone sound**, Android must allow Veyl to copy the sound apps play; it calls this "record
or cast". Tap **Start listening** again and allow it, or choose **Veyl player** or **Test
sound**, which don't need it.

### Phone sound needs Android 10

> "Phone sound" needs Android 10 or later.

Android 8 and 9 can't share other apps' sound. Use the **Veyl player** with your own music files,
the **Test sound**, or Veyl with this PC.

### The microphone can't hear high sounds

> This phone's microphone can't hear high sounds well. Turn the volume up, take off the case, or use PC mode.

The sound check heard very little of its test sound. Turn the volume up, take off a case that
covers the speaker or microphones, and run **Run sound check** again in **Settings**. You can
still try Veyl; the meter will show how well it works. Veyl with this PC is the alternative.

### Veyl stops when the phone locks

> Keep this screen on. Veyl stops if your phone locks.

> Your screen may turn off and stop Veyl. Keep Veyl open, or set a longer screen timeout.

> Veyl stopped because your screen locked. Open Veyl and tap Start.

Android stops sharing sound when the phone locks, so Veyl stops listening as soon as the screen
turns off, whichever sound you chose. To prevent that, Veyl keeps the screen on while it listens. The listening screen dims itself after 10 seconds; touch it to brighten it, or turn off
**Dim screen while listening**. When you switch to another app, Veyl shrinks to a small
picture-in-picture window, which keeps the screen on. If picture-in-picture is off for Veyl, or
your phone doesn't keep the screen on that way, keep Veyl open or set a longer screen timeout in
Android's display settings.

YouTube Music without Premium stops playing when it isn't in front. Keep YouTube Music in front
and Veyl in picture-in-picture.

After listening stops, it doesn't restart by itself: open Veyl and tap **Start listening**. With
Phone sound, Android asks again.

### No PC for 10 minutes

> Veyl stopped because no PC was connected for 10 minutes.

Veyl stops listening when no PC connects for 10 minutes. Open Veyl on the PC, then tap **Start
listening** again on the phone.

### The phone got hot

> Veyl stopped to let your phone cool down.

Android reported that the phone is very hot. Let it cool, take it out of direct sun or off a
charger, then start again.

### Capture failed

> Veyl stopped because the microphone or sound capture failed. Tap to try again.

Android stopped the microphone or the sound copy. Tap the notification, or open Veyl and tap
**Start listening**. If it happens again, restart the phone and try **Test sound** to see whether
the microphone works.

### Local network

> Veyl needs local network access to connect to your PC.

On Android 17 and later, Veyl needs your permission to accept the PC's connection on your local
network. Tap **Open settings** and allow it. Sound stays between your devices.

> Connect this phone and your PC to the same network.

The phone isn't on a local network. Connect it to the same Wi-Fi as your PC.

### Waiting for your PC

> Waiting for your PC…

The phone is ready but no PC is connected. Open Veyl on the PC. If this is the first time, pair
from the PC's **Phones** tab. If the PC doesn't find the phone, see **The PC can't find your
phone** below.

### Pairing on the phone

> Pairing didn't finish. Try again on your PC.

Start pairing again from the PC's **Phones** tab, with Veyl open in front on the phone. See
**Pairing messages** above.

### Music from the Veyl player

> No music chosen yet.

Choose music with **Choose music** or **Choose a folder**. Veyl remembers your choice without
broad access to your storage. If a file moves or an app stops sharing it, choose it again.

## Pokes are missed, or nothing happens

**Pokes are missed.** Check the meter's tip first, then [placement](placement.md). At the end of
**Test pokes**, Veyl says how many of 10 pokes it saw and gives one of these:

> Try a brighter song, more volume, or Sensitivity: High.

> Good. Sensitivity: High may catch more.

> Great.

Start each poke with your hand still, move a few centimetres toward the phone or microphone and
straight back, and leave a moment between pokes.

**Pokes appear but nothing happens.** Check that:

- actions are on: the **Actions** switch, the tray menu, or **Ctrl+Alt+V**. Actions start off
  each time Veyl starts;
- **Poke action** in **Settings** isn't **Nothing**;
- no poke test or practice step is running: actions never run during those, and those pokes are
  marked "test";
- the window you want to click doesn't run as administrator.

**The hotkey does nothing.** Another app may already use Ctrl+Alt+V. Choose another one in
**Settings → Actions hotkey → Change**. The switch and the tray menu still work.

**Pokes you didn't make.** Choose **Sensitivity: Low** or a nearer **Distance range**, keep other
movement away from the phone or microphone, and turn actions off when you aren't using them.

## The PC can't find your phone

Veyl on the PC finds phones through DNS-SD announcements on your local network. Both devices must
be on the same network, and Veyl must be open on the phone.

- **Guest networks and client isolation.** Many routers keep devices on a guest network apart, and
  some have a setting called client isolation, AP isolation or wireless isolation. They stop
  devices from seeing each other, and usually from connecting at all. Put both devices on your
  main network, or turn that setting off.
- **Add by address.** Some routers and mesh systems pass ordinary connections but drop
  announcements, for example between their 2.4 GHz and 5 GHz networks. Then the phone never
  appears under **Nearby**, but you can still connect. On the phone's home screen, read the line
  "Address:", for example `192.168.1.23:47820`. On the PC, open **Phones → Add by address**, type
  it and choose **Connect**. If the phone isn't paired yet, Veyl offers **Pair**. Your router may
  give the phone a new address later; then enter the new one, or reserve an address for the phone
  in your router's settings.
- **Firewalls.** The PC only connects out to the phone and opens no port of its own, so Windows
  Defender Firewall needs no rule. Other security software may block local connections or
  announcements; allow Veyl on your local network.
- **VPNs.** A VPN on the phone or the PC may send local traffic elsewhere. Allow local network
  access in the VPN app, or turn it off while you use Veyl.
- **Android 17 and later.** Veyl needs the local network permission; see **Local network**
  above.

## Phones that stop Veyl in the background

Some phone makers close apps in the background to save battery, for example while Veyl is in
picture-in-picture and another app is in front.

> Some phones stop apps in the background. This keeps Veyl running while it listens.

That is the description of **Let Veyl run without limits** in Veyl's **Settings**. It opens
Android's own dialog, where you decide; you can change it later in Android's settings.

Some makers add their own settings on top. Their names and places change between models and
Android versions. [Don't kill my app!](https://dontkillmyapp.com/) keeps current steps for each
maker:

| Maker                                | What to look for                                                         | Steps                                                                                                                                                                    |
| ------------------------------------ | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Samsung                              | Battery → Background usage limits: add Veyl to "Never sleeping apps"     | [Samsung](https://dontkillmyapp.com/samsung)                                                                                                                             |
| Xiaomi, Redmi, POCO                  | Autostart on for Veyl, and battery saver set to "No restrictions"        | [Xiaomi](https://dontkillmyapp.com/xiaomi)                                                                                                                               |
| Huawei                               | Battery → App launch: set Veyl to "Manage manually" with every switch on | [Huawei](https://dontkillmyapp.com/huawei)                                                                                                                               |
| OnePlus                              | Battery optimization: "Don't optimize" for Veyl                          | [OnePlus](https://dontkillmyapp.com/oneplus)                                                                                                                             |
| Oppo, realme, Vivo                   | Battery and background settings for Veyl                                 | [Oppo](https://dontkillmyapp.com/oppo), [realme](https://dontkillmyapp.com/realme), [Vivo](https://dontkillmyapp.com/vivo)                                               |
| Google Pixel and other stock Android | Veyl's app battery usage set to unrestricted                             | [Stock Android](https://dontkillmyapp.com/google)                                                                                                                        |
| Motorola, Sony, Asus, Nokia          | Battery settings for Veyl                                                | [Motorola](https://dontkillmyapp.com/motorola), [Sony](https://dontkillmyapp.com/sony), [Asus](https://dontkillmyapp.com/asus), [Nokia](https://dontkillmyapp.com/nokia) |

None of these lets Veyl listen while the screen is off; Veyl always stops then.

## Diagnostics and reporting a problem

- **On the PC:** **Help → Diagnostics** shows the connection, how many blocks of sound arrive each
  second, dropped blocks, the round trip, poke delay and the phone's battery. **Copy diagnostics**
  copies these details as text.
- **On the phone:** **Settings → Copy diagnostics** copies the app and Android versions, phone
  model, sample rate, microphone setup, sound route, privacy filter setting and the last 20 state
  changes.

Neither contains sound, pairing codes or keys. They do contain device names and models.

To report a problem, open an issue on [GitHub](https://github.com/jacobbalane/Veyl/issues/new/choose)
and paste the diagnostics. A **Device report** helps others with the same phone or PC; see
[compatibility](compatibility.md). Never attach recordings of people talking, pairing codes or
keys.
