# The sound-quality meter

Not every sound works equally well as a sonar. The meter measures, ten times a second, how well
the sound playing right now can show your hand, and suggests one thing that would help most.

You see it on the PC's **Live** tab: a score from 0 to 100, a label, four bars, the detail and a
tip. With your phone, the listening screen shows the score, the label and the tip, and the small
picture-in-picture window shows the score.

| Label    | Score      | What it means                                        |
| -------- | ---------- | ---------------------------------------------------- |
| Great    | 75 or more | The sound is well suited to sensing                  |
| Good     | 55–74      | Pokes should work                                    |
| Fair     | 35–54      | Expect some missed pokes                             |
| Poor     | Below 35   | Pokes will often be missed                           |
| No sound | 0          | Veyl can't use the sound right now; the tip says why |

The numbers are smoothed over about half a second, and the tip changes at most once a second, so
the meter doesn't flicker with every beat. When the score falls below 15, Veyl stops looking for
pokes until it recovers, so poor sound can't cause stray clicks.

The score's link to how many pokes are caught still has to be checked against real recordings;
the labels above describe the intent.

## The bars

### High sounds

How much of the playing sound is high: cymbals, hi-hats, the "s" in singing, bright synths.
Veyl senses only with sounds above 8 kHz, because the [privacy filter](privacy.md) removes
everything lower. The bar compares the high part with the whole sound, so it doesn't depend on
volume.

To raise it, play a brighter song. Pop, rock and electronic music with crisp drums usually work
well. Bass-heavy tracks, soft piano, talk and old or muffled recordings have few high sounds.

### Volume

How loud the high sounds arrive at the microphone.

To raise it, turn the phone's or PC's volume up; about half or more on a phone is a good start.
Keep the microphone uncovered and, in PC mode, close to where you poke. If the sound distorts,
the bar is held down and the tip asks you to turn the volume down a little.

### Steady

How much of the last three seconds had high sound in it. Pauses, quiet intros, gaps between
tracks and speech with breaks lower it, and Veyl can't see your hand during silence.

To raise it, play music without long quiet parts, or a playlist. The test sound is always
steady.

### Clear echo

How well the copy of the playing sound explains what the microphone hears. It is low when other
sounds reach the microphone (a TV, a second speaker, people talking, a noisy room), when the
phone's speaker or microphones are covered, or when sound effects change the sound on its way to
the speaker.

To raise it, pause other sound sources, uncover the speaker and microphones, and turn off sound
enhancers such as "Dolby Atmos" or equalizer effects on the phone or PC. In PC mode, move the
microphone closer to where you poke.

### Detail ≈ x cm

How close two things can be and still look separate to Veyl. Smaller is better. It depends on
how wide a range of high sounds is playing: with sound all the way from 8 to 20 kHz it is about
1.4 cm; music that fades out near 16 kHz, as many streams do, gives roughly 1.7 to 2 cm. It isn't
shown when there is almost no high sound. Detail has no bar of its own, but it counts toward the score.

To improve it, play brighter music, and choose a higher streaming quality in your music app if it
has one.

## How the score adds up

| Part        | Share of the score |
| ----------- | -----------------: |
| High sounds |                20% |
| Detail      |                20% |
| Volume      |                20% |
| Steady      |                15% |
| Clear echo  |                25% |

The score is 0, labelled "No sound", when Veyl can't use the sound: it isn't listening, nothing
is playing, the playing app doesn't share its sound, the sound is going to headphones (or, with
your phone, to Bluetooth), or another app is using the microphone. The tip then says which.

## The tip

The meter shows the first tip that applies, in order of importance: first whether Veyl can listen
at all, then whether sound is playing and shared, then the microphone, then volume, high sounds,
pauses and echo clarity. When nothing needs fixing, it says "Ready. Poke toward your phone." or
"Ready. Poke toward your microphone."

[Troubleshooting](troubleshooting.md) lists every tip with its fixes. The meter's measurements so
far are in [tuning](tuning.md).
