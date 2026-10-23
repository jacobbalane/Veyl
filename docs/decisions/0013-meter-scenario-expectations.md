# 0013: Match meter gates to the specified filter and tip priority

Date: 2026-10-23

The meter follows section 6.8's factors, weights and tip priority. Its PCM tests found two conflicts
with section 6.12.3. Keep the algorithms and simulator unchanged; correct those gate expectations
under section 6.12.3's provision for measured threshold amendments.

`Compressed16k` uses the specified eighth-order Butterworth low-pass at 16 kHz. A Butterworth
cutoff is its [−3 dB frequency](https://docs.scipy.org/doc/scipy/reference/generated/scipy.signal.butter.html),
not a boundary with no energy above it. The unchanged 30 dB activity mask therefore measures
10,051.7, 10,050.4 and 10,047.0 Hz for seeds 1–3 over 40 seconds, after the first five seconds.
Reported detail is 1.7 cm. Use a bandwidth gate of 9.5–10.5 kHz and its corresponding detail gate
of 1.6–1.9 cm instead of 7.0–8.5 kHz and 2.0–2.5 cm. The separate 90% poke-recall requirement
remains unchanged and pending.

`Dull` retains the specified broadband snare. Mean `high` is 0.274 in all three seeds, but the
microphone's filtered volume is low between snares. The exact priority table consequently selects
`volume_low`, then `gaps` during the brighter bursts; forcing `few_high_sounds` would violate that
table. Keep the `high < 0.3` gate. Check that each actual tip change selects the first applicable
tip, with `few_high_sounds` required when low volume does not take priority and high/detail is low.
Held tips may last up to the specified one-second minimum interval between changes.

`Gappy` reports a mean steady factor of 0.552: the raw duty fraction and the piecewise-linear
steady factor are different quantities. Its original `steady < 0.6` gate needs no amendment.
Blocked capture, silence, clipping and route checks retain their original requirements. Source
status starts off, so unconfigured streams cannot confirm pokes. Unsafe source changes zero the
score immediately; their tips may update immediately rather than retaining an obsolete ready tip.
