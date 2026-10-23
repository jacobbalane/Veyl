# 0018: Align through bounded short gaps

Date: 2026-10-24

The full dropout audit loses two seed-2 pokes to late startup: the first lock arrives at
5.85 seconds, and settling removes the following approach. Requiring every frame in the
one-second reference and wider microphone search span to be present delays alignment even
when only one 10 ms block is missing. Gesture-window validity and alignment evidence serve
different purposes.

Allow GCC-PHAT to use zero-filled holes covering at most 2% of each search input, with no
continuous hole longer than 10 ms. Both limits are configurable and can be set to zero. The
requested span must stay within the stream's received and retained bounds; unreceived or
overwritten audio is never sparse evidence. Keep the existing band-level, PSR, measured-peak
continuity and two-estimates-a-second-apart requirements. Bound failed searches by the normal
alignment attempt interval too.

Rejecting excessive missing coverage clears a pending alignment candidate, just as a failed
GCC estimate does. A good estimate before a rejected search cannot confirm a later one; two
fresh estimates are required.

This changes only the lag search. Every processing window touching a gap remains invalid,
clears pending gestures and retains the specified 300 ms suspension. No missing frame becomes
a valid echo or gesture observation. Long gaps still prevent an alignment estimate.

Focused tests cover both supported sample rates, signed lags, sparse unrelated input, long
gaps, disabling tolerance and exact missing-frame coverage. Complete gesture acceptance and
target-hardware performance remain separate checks.
