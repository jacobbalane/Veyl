# 0014: Keep sensing separate from offline labels and measurements

Date: 2026-10-23

The lab replays the recorded PCM, frame numbers, timestamps and flags through a fresh portable
engine. Mode, source, route and source-event changes come from control metadata. Labels are read
only after replay finishes; changing them must not change features or detections. Tests compare
repeated reports and sensing output before and after removing labels.

Match truth within ±150 ms and human marks within the specified −600/+200 ms window. Sort label
windows by their upper bound, then assign the earliest remaining feasible turn. This gives a
deterministic one-to-one matching without losing matches to a greedy nearest-label choice.
Pool counts and raw delay samples across sweep inputs; do not average per-recording medians or F1.
Percentiles interpolate between sorted samples using position `(n − 1) × p`.

Report simulator latency as detection time minus the matched true turn. Human marks include an
unknown reaction delay, so they cannot establish that latency: report their mark-to-estimated-turn
offset separately. Confirmation time, from the engine's estimated turn to its detection, is always
separate. Keep signed offsets. Undefined measurements are null in JSON and empty CSV cells.

No marks in a real recording do not establish that no gestures occurred. Precision and false-poke
rate remain undefined in that case. A known still recording can explicitly set
`device.annotations_complete = true` in its metadata. Provided human marks otherwise imply complete
annotation; setting that field false declares partial annotation. Generated scenes have complete
truth, including a negative scene with zero expected pokes. These declarations affect reporting,
never sensing.

Analysis writes numeric JSON/CSVs and a motion PNG. It creates no audio copy and refuses to replace
existing reports. The PNG uses microphone time and echo distance, with separate truth/mark and
detection markers; grey denotes unavailable or settling features. Sweeps collect summary metrics
without retaining full traces, and keep only one recording in memory at a time.
