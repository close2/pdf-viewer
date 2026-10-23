# ADR 2197 — Visible share does not scale a whole graphics frame

Status: **accepted** at the merge that took it, `b9d05ee8` (line added by ADR 2260).

A zoom-dependent number is not necessarily a better prediction. `FrameCost` already records
scene, encode, upload and execute phases plus command culling, but those facts arrive after a
real graphics frame. They do not describe a new view that only the processor has drawn.
`DisplayList::share_reaching` cheaply samples at most 4096 top-level commands before a draw;
it does not separate fixed scene/encode work from visible geometry, pixel or transfer work.

Do not multiply the whole old GPU duration by that geometric share, and do not add an automatic
GPU probe merely to update the displayed number. Either would answer a different question while
claiming a current-view cost. ADR2196 instead distinguishes compatible samples from seeds.
The existing seeds can remain constant through a gesture, and the display says they are seeds.

A future normalized predictor needs measured phase-specific work and uncertainty, including
retained lines, groups, images and coverage-lane changes. Its acceptance is end-to-end latency
on the actual launch/turn/view paths, with source provenance and a quiet device. A lower estimate
or an unchanged pixel digest alone does not establish that it chooses a faster lane.
