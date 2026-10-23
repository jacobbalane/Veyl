# 0017: Require sufficient motion at the poke turn

Date: 2026-10-23

Hand presence remains true for six hops below its on threshold. That hysteresis makes the display
stable, but it also let a weak, changing echo become the minimum-distance point of a confirmed
poke. Most false hover detections had motion below the selected on threshold at their turn.

Require `turn.z >= z_on` as well as the existing approach, retreat, depth, speed, duration and
valid-hop requirements. The selected sensitivity supplies `z_on`; other thresholds and the
confidence formula keep their values. Test weak turns at all three sensitivities. This is a
stricter detector guard, not a relaxation of any acceptance criterion.

The basic-phone and full clock regressions still pass. A full-duration development matrix now
replays 18 scenarios for each seed through the lab, with control metadata and post-replay label
matching. It prints measurements and failures, and exits unsuccessfully when a gesture gate
fails. It does not claim the completed CI matrix, stereo-mode verification or meter acceptance.

The current candidate has 30/30 basic matches and 60/60 positive-clock matches without extras.
Hover produces 0, 2 and 0 false pokes (the seed-2 limit is one). Varied heights, typing with pokes,
dropouts, compressed audio, stereo and PC recall still fail. Volume-step results also miss two
seeds. See `docs/tuning.md` for all measured counts. Raw-peak jump and window-leakage prototypes
were discarded because they rejected real pokes or became unstable. No gesture gate changed.
