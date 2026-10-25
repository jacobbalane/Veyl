# How it works

Veyl turns a poke in the air into a click by listening to echoes of the sound you already play.
It needs no camera, no extra hardware and no special tone.

## The idea

When your phone or PC plays music, the sound leaves the speaker, bounces off everything nearby
and returns to the microphone. It arrives first straight from the speaker, then again a moment
later from the desk, the monitor, and your hand. Each echo arrives after a delay that depends on
how far the sound travelled.

Veyl knows exactly what was played. On a phone, Android shares a digital copy of the sound that
apps play. On a PC, Windows shares the sound sent to the speakers. Comparing that copy with what
the microphone hears tells Veyl how strongly sound comes back after each tiny delay. That is an
echo map, and a delay is a distance: about 3.6 mm per sample at 48 kHz.

Music changes all the time, but the room does not. So Veyl can learn the echo map from any
reasonably bright song, and the same map comes back every moment unless something moves.

## From echoes to a poke

1. **Find the timing.** Veyl first lines up the copy and the microphone, then follows any slow
   drift between their clocks, so a millisecond of delay always means the same distance.
2. **Learn the room.** Echoes that stay put, such as the desk and the monitor, become the
   background. Veyl predicts what the microphone should hear from the background alone.
3. **See what changed.** What the prediction does not explain is change. Veyl measures it over a
   short stretch of about 21 ms, so a quickly moving hand still shows up sharply, and only in the
   high sounds that Veyl is allowed to hear.
4. **Follow the hand.** A strong, concentrated change is a hand. Veyl follows its distance from
   moment to moment and ignores brief bursts elsewhere.
5. **Recognise a poke.** A hand that moves quickly closer, turns, and moves back is a poke. Veyl
   needs about 3 cm toward the phone and 2 cm back on the Normal setting. A poke is confirmed a
   little after you pull back, typically about a tenth of a second after the turn.

Typing clicks, knocks and short dropouts in the audio are recognised and left out, rather than
mistaken for a hand. Pauses in the music suspend detection until sound returns.

## The sound-quality meter

Not every sound works equally well. Veyl measures how much of the sound is high and bright, how
loud it reaches the microphone, how steady it is and how clearly the copy explains what the
microphone hears. The score and tip tell you what would help, such as a brighter song or more
volume.

## Privacy

Before anything leaves your phone, Veyl removes the low sounds where voices are. Sound travels
only to your paired PC over an encrypted link. In PC mode it never leaves the PC. Veyl saves no
sound unless you start a test recording on the PC, which stays on that PC. See [privacy](privacy.md).

## Limits

Veyl is a sonar made from music, so it needs something playing with enough high sound in it.
Headphones, Bluetooth speakers that add delay, covered microphones and very quiet or muffled music
make the echoes harder to read; the meter says which applies. Very fast pokes right after holding
your hand still for a long time are the hardest case for the current version.
