# Related work

Veyl builds on years of research into sensing with sound, and on a few classic signal-processing
methods. This page credits that work and says how Veyl differs. These are other people's results
on their own systems; none of them is a measurement of Veyl.

Veyl's own choices: it uses the music or video already playing instead of adding a tone, it gets
an exact digital copy of that sound from Android or Windows, it shows a live meter of how well the
current sound works, and it keeps sound on the user's own devices, filtered, paired and visible.

## Acoustic sensing

| Work                                                                                                      | What it showed                                                                                                                         | How Veyl differs                                                                               |
| --------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| FingerIO (Nandakumar, Iyer, Tan and Gollakota, University of Washington and Microsoft Research; CHI 2016) | A phone or smartwatch used as an active sonar tracks a finger in two dimensions to about 8 mm on average, using inaudible OFDM signals | Veyl uses the music already playing, and controls a separate PC                                |
| LLAP (Wang, Liu and Sun, Nanjing University; MobiCom 2016)                                                | Device-free gesture tracking with millimetre-level movement from the phase of acoustic signals on ordinary phones                      | Veyl plans the same phase idea, applied to music, for finer distance in v0.2                   |
| Strata (Yun, Chen, Zheng, Qiu and Mao, University of Texas at Austin; MobiSys 2017)                       | Fine-grained device-free tracking by estimating the channel impulse response, which separates paths with different delays              | Veyl estimates the same kind of impulse response, its echo map, from whatever sound is playing |
| SoundWave (Gupta, Morris, Patel and Tan, Microsoft Research and University of Washington; CHI 2012)       | In-air gestures from the Doppler shift of an inaudible tone, with a laptop's own speaker and microphone                                | Veyl's PC mode uses the same kind of hardware, but senses with the playing sound, not a tone   |
| AudioGest (Ruan, Sheng, Yang, Gu, Xu and Shangguan; UbiComp 2016)                                         | Hand gestures detected from Doppler-shifted echoes with a device's built-in speaker and microphone                                     | Veyl adds a phone as the sensor for a PC, music as the signal, and a live meter                |
| EchoTrack (Chen, Li and Wang; INFOCOM 2017)                                                               | Device-free hand tracking on a phone from the time of flight of chirps from its speakers                                               | Veyl works with music instead of chirps                                                        |
| CovertBand (Nandakumar, Takakuwa, Kohno and Gollakota, University of Washington; IMWUT 2017)              | Sonar pulses hidden in music can track people's movement in a room, a warning about the privacy risk of acoustic sensing               | Veyl is visible, paired, filtered and local by design; see [privacy](privacy.md)               |
| AcouLetter (2026)                                                                                         | Letters written mid-air with a thumb, sensed with phone acoustics and motion sensors, also while music plays                           | Veyl targets PC control and uses the music itself                                              |
| Elliptic Labs                                                                                             | Commercial ultrasound sensing that runs on devices' built-in speakers and microphones                                                  | Veyl is open source and installs on devices people already have                                |
| Webcam gesture apps, such as Touchless and Neural Lab AirTouch                                            | Touch-free PC control with a camera                                                                                                    | Veyl uses no camera                                                                            |

## Methods Veyl uses

| Method                                                | What it is                                                 | Where Veyl uses it                                                     |
| ----------------------------------------------------- | ---------------------------------------------------------- | ---------------------------------------------------------------------- |
| GCC-PHAT (Knapp and Carter, 1976)                     | Time-delay estimation by phase-transform cross-correlation | Lining up the copy with the microphone ([echo engine](echo-engine.md)) |
| Audio EQ Cookbook (Robert Bristow-Johnson)            | Formulas for biquad filters                                | The privacy filter's four high-pass sections, on the phone and in Rust |
| One Euro filter (Casiez, Roussel and Vogel; CHI 2012) | Low-latency, low-jitter filtering for pointing             | Planned for the pointer on the virtual screen in a later version       |

## Sources

- FingerIO: [project page and paper](https://fingerio.cs.washington.edu/).
- LLAP: [doi:10.1145/2973750.2973764](https://doi.org/10.1145/2973750.2973764).
- Strata: [doi:10.1145/3081333.3081356](https://doi.org/10.1145/3081333.3081356).
- SoundWave: [Microsoft Research publication page](https://www.microsoft.com/en-us/research/publication/soundwave-using-doppler-effect-sense-gestures/).
- EchoTrack: [paper](https://cis.temple.edu/~yu/research/EchoTrack-InfoCom17.pdf).
- CovertBand: [doi:10.1145/3131897](https://doi.org/10.1145/3131897).
- AcouLetter: no public link found yet. Corrections are welcome.

If your work belongs here, or a credit is wrong, please open an issue.
