# 1449 — A neighbour stretches a walk by a fifth at most

Slot 6 of batch seventy, 2026-10-08, a measure round. ADR 1734; no row moved, no question.

**Premise.** It held: ADR 1706 section 5 names the figure and no line of `/home/AI/heavy-walk.log`
times one walk alone and beside a neighbour in one sitting. The 64 `batch=batch-1438-1443` lines
were read through `tools/state.sh gates-cost` before the arms export's own line landed.

**The sitting.** Twelve `--clock` holds, 12:21 to 12:37, after the arms export's `done` at 12:20.
Each walk pinned to the eight fast CPUs with a 30 ms spin (ADR 1577), binaries copied out with an
`md5sum`, A B / B A / A B. Beside: `pdf-model --test corpus` looped at `--tree 6`, 4 threads.
`raster_golden`: 18.79, 19.90, 18.79 s alone, 21.01, 21.01, 19.91 s beside: 1.060 min to min,
1.118 median to median. The Tier 1 column: 122.61, 102.76, 104.96 s alone, 120.46, 123.77,
116.03 s beside: 1.129 and 1.148, worst pair 1.204, against an alone spread of 1.193. Processor
time rose 1.106 for the column. The neighbour: 6.64 to 7.75 s alone, median 9.10 s beside the
column (1.175), one iteration 12.94 s (1.672). **No hold stretched 1.5×; the lane stays**, and
nothing goes to slot 5. Batch sixty-nine's replay: one lane minus built, 22 801.8 s.

**Found.** Every pinned `raster_golden` failed, 21 first pages "raster only"; the same binary passed
unpinned at 4 and 8 threads and failed pinned at 4. `plan_strips` takes the strip count from
`available_parallelism` and ADR 0219 says pixels depend on it, so the golden holds a 24-CPU
process's pixels. Proposed in the report: `with_strips` in the gate, regenerated (ADR 1734 §4).

**Gates cost, batch sixty-nine.** 32/33 green, 1 783 s. Dearest by wall `t3-vfs_read`, 188 s; by
wait `t2-xmp`, 1 s, the only gate that queued. `t3-accessibility` exited 101 in the gate log and its
re-run at 11:40 exited 0. The rounds queued 4 800.9 s, the dearest wait 1441's `jpeg2000` at
1 133.8 s. Dev-profile `cargo test` lines under the lock: 0, 0.0 s of queue. The sitting held both
lanes 16 min; 1444 queued 865.4 s and 1445 364.3 s behind it.

**Unfinished.** The neighbour's alone runs were taken after the sitting, not interleaved. The
golden's strip count is `pdf-model`'s to fix. `doc/todo/42` (the launch path) needed no edit.

**Gates.** `cargo test -p conformance`: exit 0, 417 passed in 55 binaries. `tools/state.sh
gates-cost`: exit 0. Twelve timed `--clock` holds, exit 0 (the six
column runs 0, the six pinned golden runs 101 for ADR 1734 §4's reason); one neighbour-alone hold, exit 0; three
golden controls, exit 0, 0 and 101 (the pinned one). No band, ratchet or held list was re-taken.
