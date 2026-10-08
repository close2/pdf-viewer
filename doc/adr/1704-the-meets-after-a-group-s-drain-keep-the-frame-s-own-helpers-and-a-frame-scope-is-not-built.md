# 1704 — The meets after a group's drain keep the frame's own helpers, and a frame scope is not built

Session 1434. Status: **accepted**; nothing is built. Answers ADR 1692's "Unfinished"
(`bug1721218_reduced.pdf` still starts two sets) and its section 4 (the photograph's 23 threads).
Keeps ADR 0023's shape, ADR 1686's decision (no pool of the frame's own) and ADR 1692's lending.
Context: ISO 32000-2 §10.7.4, §11.4; `CLAUDE.md` principle 2 and the rule on optimisations; ADRs
0023, 1541, 1686, 1692; traps 66, 101, 110; habit 75. Code read:
`raster/crates/raster-gpu/src/encode/parallel/commit.rs` (`drain`), `encode/layer.rs`
(`plan_child`), `encode/meet/helpers.rs`, `encode/meet/deferred.rs`, `raster/reduce.rs`
(`area_averaged_cells`).

Every figure is the 890M through RADV, `examples/frame_budget` built `--release` from exports of
HEAD (`e833db88`), each in its own target directory (`md5sum`-distinct), five rounds a run and the
minimum of each, arms interleaved run by run behind the lock (`--clock`), started at a one-minute
load under 8. "Pinned" is `turn_path`'s eight fast CPUs and eight threads, "unpinned" the 24 a
viewer gets here. Milliseconds; ranges are over runs.

## 1. Where the second set is

A probe of one turn (pinned) reads the page's group as two frames. The chromatic frame's walk queues
3 024 residue-clipped dots and drains them at weight 15 120: the fan-out lends its 7 threads, and
the commit records all 3 024 meets (10 119 cut pixels). The drain is `plan_child`'s, at a layer
boundary (`encode/layer.rs`: a queued mark belongs to the plan that was current when the walk
reached it). Two layer boundaries later a drain of 232 more dots weighs 1 160, under the 4 096
floor, so it runs on the walk's thread. Its first meet finds nobody claiming and starts the frame's
own 7 helpers, which make its 232 meets (761 pixels). The black frame drains the same marks and
records no meet, every one kept from the chromatic render. So the turn's 37 are 7 + 7 + 8 ramps for
the chromatic frame and 7 + 8 for the black one. Unpinned that is 114. The step's 21 (69) are the
same less the ramps.

## 2. Recording those meets before the drain they follow: not taken

They are a later layer's marks. To record them in the first drain's commit, the queue would have to
cross the layer boundary that `plan_child` drains at. That is the one place the walk's current plan
moves, and it is how the commit's order stays the walk's. The prize is bounded by what that drain
does off the meets. Its 232 jobs rasterise in 0.29 to 0.30 ms of one thread (246 to 260 ns a
segment, no more than a plain fill's). Then 1.1 to 1.3 ms of its commit passes before the first
meet, and a commit is the walk's thread at any thread count (`parallel`'s third phase). The meets'
areas go to helper threads beside the walk either way. So the reordering would save the second set's
start and at most 0.3 ms of rasterising. That is section 3's prize, and section 3 measured it.

## 3. A set the frame keeps while the walk runs: measured, not built

Built in an export of HEAD, 137 lines of diff, and not kept in the tree. The walk runs inside a
`std::thread::scope` of the frame. A drain that lends starts its fan-out on that scope, with its
jobs in an `Arc`. Each of its threads hands back its masks and then claims meets until the settle
closes the frame's helpers, so nothing waits at the drain. The prototype passed `raster-gpu`'s
`encode_threads` (9), `exact_meets_settled_after_the_walk` (1) and `encode_thread_sets` (1).

| `bug1721218_reduced.pdf`, pinned, 7 runs, load 5.4–7.2 | threads | HEAD | prototype |
|---|---|---|---|
| turn | 37 → 30 | 142.22–159.19 (median 142.71) | 142.34–156.63 (144.40) |
| seventh | 37 → 30 | 140.67–148.78 (143.45) | 141.75–151.54 (143.67) |
| step | 21 → 14 | 81.98–91.22 (83.23) | 81.83–96.05 (83.24) |

On the Type 3 page, same runs, threads stay 7: turn 10.09–11.09 against 9.73–33.69, seventh
9.69–10.44 against 9.88–33.16, step 10.62–11.47 against 10.52–17.85. The prototype's high figures
are its first two runs, the second about three times every row. Its medians are 10.37, 10.13 and
11.59 against 10.72, 9.98 and 11.21. Unpinned (5 runs, load 7.0–7.9), `bug1721218_reduced.pdf`
starts 91 against 114 on the turn, which reads 153.90–196.53 against 152.22–186.68. The step starts
46 against 69 and reads 95.52–119.73 against 96.14–124.53.

**What the threads cost, measured where they start.** Starting 7 threads took 0.07 to 0.30 ms
(median 0.094, 45 starts) and 23 took 0.29 to 1.34 (median 0.49). Those are the reduction's starts
(section 4), on the same machine. So the prize is 0.1 ms of a 142 ms pinned turn and 0.5 of a 153 ms
unpinned one, inside every row's spread. **Not built**, for ADR 1686's reason. Its cost is the walk
inside a frame-wide scope, plus a lifetime-carrying spawner on `Encoder`, for a figure the benchmark
does not hold.

## 4. The photograph's reduction: its own threads, and no decision

A census of `turn_path`'s eleven pages, one probed turn each (pinned), finds no frame that both
drains past the fan-out's floor and reduces an image. `issue12841_reduced.pdf` reduces once a turn
(48 bands), `22060_A1_01_Plans.pdf` four times and `images.pdf` five, and none of them drains a
fan-out. The reduction runs when the device realises the image, after the encode has returned. So
there is no fan-out to share, and sharing would be ADR 1686's frame pool. Its spawn against its own
work, `issue12841_reduced.pdf` unpinned (load 6.8–7.9, 30 to 60 reductions an arm):

| reduction threads | 1 | 4 | 8 | 12 | 24 |
|---|---|---|---|---|---|
| reduction, median | 4.44 | 1.51 | 1.33 | 1.21 | 1.26 |
| of it, starting the threads, median | — | 0.055 | 0.097 | 0.20 | 0.49 |
| turn, budget | 30.95–33.62 | 27.94–28.65 | 28.50–29.85 | 28.05–42.83 | 27.67–31.12 |

At 24 threads starting them is a third of the reduction's wall, and the threads past eight buy about
what starting them costs. Capping it at 4, 8 or 12 moved no row of the turn, so no cap is set.
Splitting the rows at all is worth about 3 ms of a turn's transfer: 1.76–1.81 against 4.66–5.11 at
one thread.

## 5. What would reopen it

A host with more processors than 24, where 23 starts stop being inside a row's spread. Or a frame
pool built for another reason, which would carry both sets at no cost of its own. Or a page whose
meets after a drain are most of the frame's, read by the probe's split of section 1.
