# 2719 — On lavapipe the zoom-step test cannot exercise the race it is about

Status: accepted (a finding; nothing built). Session 1952. Context: ADRs 1289, 2675, 2718.

## What was measured

ADR 1289's test is about a zoom step sent while the *first* view's sharp pass is waiting out its
stillness. That pass exists only where the first frame was built in at most 100 ms. Printed by the
test (`--nocapture`), on this VM:

| adapter | load | first frame built | step built | race exercised |
|---|---|---|---|---|
| lavapipe (llvmpipe, Mesa 25.0.7) | alone, 4 processors | 202–209 ms (5 runs) | under 100 ms | no |
| lavapipe | 4 suites on 4 processors | 0.95–1.08 s (40 runs) | 61–241 ms | no |
| RX 580 (RADV), through `tools/hang-capture.sh` | the suite under nextest | 89.7 ms | 25.7 ms | yes |

So CI, which has only lavapipe, never exercises the race: the first view's pass is declined, the step
has nothing to overtake, and the order asserted holds trivially. The test now says so on its output
("the first view's sharp pass was declined … the step had no pass to overtake").

## Decision

The test is kept as it is: it is exact where the race exists (the owner's devices and the RX 580) and
correct, though not discriminating, where it does not. Making it discriminate on lavapipe would mean a
smaller page or window chosen to land under 100 ms on one software adapter — a constant fitted to a
machine — or a seam to set the thread's prediction, which changes the product to suit a test. Neither
is taken without a measurement of what the first frame's 200 ms is made of; that is owed, not done.
