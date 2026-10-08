# 1435 — The second lane stays

Slot 4 of batch sixty-eight, 2026-10-08, a measure round. ADR 1706; no row moved, no question.

**Premise.** It held. `grep -c 'batch=batch-1426-1431' /home/AI/heavy-walk.log` gave 128 at the
start: 92 rounds' lines, the arms export's and 35 of the merge's, every one with `kind=` and
`lane=`. The kept scripts were batch sixty-four's, with its unlogged holds and its clock runs
written in, so the replay is new (`scratchpad/r1435/`, kept at `/home/AI/lock-replay-1435/`).

**The replay.** Each walk keeps its observed hold and its declared kind, and is asked the observed
gap after its round's latest ask, grant or release. One lane: 37 801.9 s of the rounds' queue.
The built rule: 14 229.2 s. **Difference 23 572.7 s** (33 052.2 s when chained on asks and releases
only), against ADR 1684's 3 600 s. The lane stays, and nothing is handed to slot 6.

**Calibration** (trap 13). On batch sixty-four's single-lane log, the one-lane arm gives 25/25 waits
within 1 s, 16 594.2 s against 16 593.1. The two-lane arm at observed asks gives 12 622.7 s against
13 505.5 s observed, but only 37/93 lines within 1 s. That is the pollers' race: a wrapper started
inside the half-second poll beats one queued for minutes, and whole-second ask times cannot replay it.

**The blind spot.** 74% of the rounds' hold time was beside another hold. The drive took 5.73 s a
work beside lane-2 walks, against 4.54–5.44 s in the three single-lane drives: 1.05–1.26×. The two
`script_corpus` gates came to 0.98–1.02×. The merge's 33 gates, alone in both batches, drifted
0.83–1.20× (median 1.000). The lane comes out only at a stretch of 1.5–3×.

**Gates cost, batch sixty-seven.** `tools/state.sh --round 1435 gates-cost`: 35/35 green, 1 757 s.
The dearest by wall is `t3-vfs_read`, 181 s. By wait, four gates tie at 1 s, because the merge ran
alone. In the rounds' lock lines the dearest wait was 1428's `cargo build --release`, left large:
2 239.5 s behind the arms export, for a hold of 1.1 s. Declared small, it and `raster-examples`
save 1 934.8 s on the replay. `cargo test -p conformance` under the lock waited 1 820.8 s.

**Unfinished.** No walk was timed beside a neighbour and alone in one sitting. ADR 1706 section 5
makes that the re-ask, at over 1.5×. Two gate pairs (`corpus`, `dates`) hold a build the log does
not separate.

**Gates.** `cargo test -p conformance`: exit 0, 408 passed in 54 binaries (407 and 1 failed,
`records.rs` on this record, before this paragraph had its figures). `tools/state.sh gates-cost`:
exit 0. Nothing heavy ran; no band was re-taken.
