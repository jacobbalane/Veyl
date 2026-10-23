# 0012: Measure model variance and reject jumps between echoes

Date: 2026-10-23

The first complete-engine tests detected none of the ten basic pokes with the original defaults.
A 40 ms complex average suppresses the changing phase of a moving return. Shortening it to 5 ms
helps a single speaker, but stereo estimates add enough variation to hide that return. Input
channel coherence alone does not identify a physical speaker that plays an equal mix of L and R.

Add an opt-in `infer_single_path` experiment. After one second of stereo observations, compare
the two transfer paths averaged with coefficient 0.01. If their band-weighted difference energy
is less than 1% of their combined energy, use their mono mix until the estimator is reset.
Different stereo paths keep stereo mode. Keep coherence smoothing at 40 ms independently of the
fast transfer time. Defaults retain the original 40 ms estimate and disable physical-path inference
until the remaining scenarios pass.

During settling, estimate complex model variance with a 150 ms average. Adjacent-hop differences
underestimate variation when windows overlap. Include the measured L/R cross term in both the
motion energy and its floor. Limit a changed tap's energy to its current echo energy so a vanished
background return cannot alone dominate a new return. The existing relative floor stays unchanged.
Use decision 0011's measured-peak continuity for the background distance origin as well.

The poke detector implements the specified approach, turn, retreat, validity and refractory rules.
Reject a trajectory step above 500 cm/s: switching to a different echo must not become an approach.
This limit exceeds the fastest specified simulated hand motion. Range or sensitivity changes,
pauses, gaps and source changes discard pending trajectories. The meter and source state gate
confirmation; they must be implemented before the complete detector can be accepted.

The candidate detected 30/30 basic pokes with no extras across seeds 1–3 before meter integration.
Still, wave, typing and walking negative runs produced no pokes. Hovering produced 0, 8 and 1 false
pokes over 300 seconds for seeds 1–3. PC and drifting gesture recall remain insufficient. These
are research results, not acceptance of M4.5 or a change to any gesture gate. Failed covariance
whitening, stronger ridge and adaptive-cancellation prototypes were removed from the engine.
