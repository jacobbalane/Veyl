# Acoustic simulator

The simulator creates reference and microphone audio from sound paths, room noise and moving
reflectors. It supplies the same filtered PCM and block metadata that the desktop receives.
Labels describe the actual trajectories; they do not determine engine detections.

List the 21 available scenes, then generate one into a new or empty folder:

```powershell
cargo run --release -p veyl-lab -- scenarios
cargo run --release -p veyl-lab -- sim poke_basic --seed 1 --out .cache/sim/pb
cargo run --release -p veyl-lab -- analyze .cache/sim/pb
```

Analysis now writes stream and engine statistics, feature/meter/poke CSVs and `motion.png`.
The current defaults still miss gestures; an experimental phone profile passes the basic case
but not the complete scenario matrix. See [offline analysis](lab.md) for profiles and sweeps.

Use `--seconds 2` for a short input; omit it for the scenario's specified duration. Acceptance
runs use the full duration and seeds 1, 2 and 3. Seeds, acoustic geometry and trajectories are
deterministic. An export's creation timestamp identifies the run and is not a sensing input.

The output contains `meta.json`, `ref.wav`, `mic.wav`, `blocks.jsonl`, `labels.jsonl` and an empty
`events.jsonl` reserved for live engine events. WAVs are 16-bit PCM after the 8 kHz privacy filter.
The metadata contains simulator source events, the scenario and seed; it contains no recorded
music. Exports refuse to overwrite a nonempty folder. Keep generated output under `.cache`.

Each speaker has its own channel mapping, high-pass/high-shelf EQ and soft clipping. The model
adds direct and static paths, a diffuse room response and distance-dependent hand reflections.
Eight-tap fractional delays follow trajectories every sixteen samples. Microphone noise and
ambient typing, fan or speech precede clipping and filtering. Lost blocks retain absolute frame
positions and flag the next received block. Microphone timestamps follow the simulated clock.

Bright, Dull, Compressed16k and Gappy variants share the generated test sound. Gappy has one
second of silence every four seconds, as specified. PC scenes place two speakers forty
centimetres apart, use separate clocks at 150 ppm and put the hand twenty-five centimetres from
the microphones. The Bluetooth scene adds 350 ms output latency. These are simulated conditions;
they do not establish real hardware performance.

To stream a scene to the Windows app through the real encrypted Link:

```powershell
cargo run --release -p veyl-lab -- fake-phone --scenario poke_basic --seed 1 --realtime
```

Use Add by address with `127.0.0.1:47820` and compare the codes in an interactive terminal and
Veyl. The fake phone listens only on loopback. Pairings last until the process exits. It plays no
audio through hardware and never captures a microphone.
