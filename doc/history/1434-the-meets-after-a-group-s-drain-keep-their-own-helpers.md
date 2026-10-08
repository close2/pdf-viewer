# 1434 — The meets after a group's drain keep their own helpers

Slot 3 of batch sixty-eight, 2026-10-08, a pixels round. ADR 1704; ADR 1705 and Q330 not used. No
ledger row moved (§10.7.4's row is `departed` and unchanged: no code changed, no byte moved).

**Found** (ADR 1704 section 1). A probe split `bug1721218_reduced.pdf`'s pinned turn of 37 threads
into 7 + 7 + 8 ramps (chromatic frame) and 7 + 8 (black frame). The 232 meets after the drain are
recorded by a drain of their own, which a later layer's boundary forces (`plan_child`). It weighs
1 160, under the 4 096 floor, so it runs on the walk's thread and its first meet starts the helpers.
**Not taken** (section 2): recording them in the first drain means a queue across a layer boundary.
That drain's own rasterising is 0.30 ms of one thread, so the prize is section 3's.
**Measured, not built** (section 3): a scope of the frame, in which a lending drain's threads stay
with the meets until the settle. Prototype in an export, 137 diff lines, bytes held by
`encode_threads`, `exact_meets_settled_after_the_walk` and `encode_thread_sets`. Pinned, 7
interleaved runs, threads 37 → 30 on the turn and 21 → 14 on the step. Turn 142.34–156.63 ms against
HEAD's 142.22–159.19, step 81.83–96.05 against 81.98–91.22, Type 3 rows inside HEAD's spreads but
for two runs. Seven starts cost 0.094 ms (median), so the prize is inside every spread.
**The reduction** (section 4). No `turn_path` page both drains a fan-out and reduces an image.
Starting the reduction's 23 threads is 0.49 ms of its 1.26 ms at 24 threads, and 0.097 of 1.33 at
eight, against 4.44 ms on one thread. Caps of 4, 8 and 12 moved no turn row: no decision.

**Premise.** Held: 232 meets after the drain, 37 (114) threads, 48 bands over 2 359 296 samples and
23 (7) threads. Not held: that those meets are "in the walk" after the drain. They are in a later
drain's commit, which has no threads because it is under the floor.

**Unfinished.** Nothing built, so nothing to merge in `raster/`. `doc/checks/turn-path.toml` is
unchanged, and no figure of it moved.

**Gates.** No `.rs` file was touched, so rustfmt, clippy and nextest are not owed, and neither are
the six arms, `raster_golden` and `turn_path`: there is no change to compare against
`/home/AI/arms-1432/`. Prototype in its export: `cargo test -p raster-gpu --test encode_threads
--test exact_meets_settled_after_the_walk --test encode_thread_sets`: exit 0, 9 + 1 + 1 passed.
`cargo test -p conformance`: exit 0, 412 passed in 54 binaries.
