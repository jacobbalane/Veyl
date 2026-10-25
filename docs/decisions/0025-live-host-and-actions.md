# 0025: How the live engine host reads sources, times pokes and runs actions

Date: 2026-10-25

M5.1 and M6 connect the echo engine to real sources and to Windows input. §7.6 and §7.7 leave a
few details open; these are the choices.

**Routes.** A phone reports `unknown` when Android lists no output device. Nothing then suggests
headphones, so the engine treats it as the speaker; the meter's other factors still judge the
echo. PC mode now reports its speaker type as the route (`headphones`, `bluetooth` or
`speaker`), which the engine's PC safety rules and the `pc_bluetooth` tip already expect. A phone
capture reason of `mic_silenced` or `mic_error` sets the matching source status.

**Engine lifetime.** Each `audio_config` starts a new engine with the tuned defaults, as the lab's
`analyze` does: PC mode, the privacy filter's cutoff as the lowest frequency and the wider PC
distance limit. Capture stopping resets it, because audio resumes with new frame numbers. A
temperature change restarts the engine, which fixes its distance scale at creation; sensitivity
and range change live.

**Live frames.** They follow microphone audio time, 30 per second, rather than a wall clock, so a
stalled source sends nothing instead of empty columns, and a burst after a stall skips to the
newest frame. Only the live frame reads the engine snapshot, because reading it hands over the
accumulated motion row.

**Latency.** §7.6 measures from the poke's turning point to when the action ran. The turning point
is placed on the PC clock through the latest microphone block: phone blocks carry their transport
delay from the PING offset; PC blocks reach the queue when complete, one block after their first
frame. The end is when the engine thread hands the action to the actions worker, which performs
it within a few milliseconds; the worker reports only failures. Values outside 0–5 s are ignored.

**Actions.** A poke acts when actions are on, no poke test or onboarding practice runs and the
action is not "Nothing". Windows does not report input it drops for a more trusted window (UIPI),
so before `SendInput` Veyl compares the integrity level of the target (the window under the pointer
for a click, the foreground window for keys) with its own, and also treats fewer inserted events
than requested as blocked. Either shows "Windows blocked the click in an app running as
administrator."

**Settings.** Changes from the interface are validated, applied to the latest saved settings and
saved atomically; side effects (hotkey, start with Windows) run first and are undone if a later
one fails. Detailed logs apply from the next start, since the log level is set once at startup.

**Development builds.** `veyl-dsp` and `rustfft` are optimized in the dev profile. Without it the
engine processed 16 s of audio in about 25 s, so `tauri dev` could not keep up with a live source.
