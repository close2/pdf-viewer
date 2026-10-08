# 1472 — The first long hold read at its start, and the thread prefix left to the wrapper

Slot 5 of batch seventy-four, 2026-10-08, an instruments round. No ADR (no rule changed), no row.

**ADR 1756's prediction, over the lines there were.** At 23:33 the log held 4 lines saying
`batch=batch-1468-1473` (the last read asked 23:12:20; the arms export's, 2 189.9 s on lane 1, came at
23:23). One is `kind=long`: round 1471's seed census, 1 170.8 s on lane 2 from 23:00:59, queued 182.0 s
behind round 1470's small walk. One run queued behind it: round 1468's `raster_golden`, 490.8 s,
`behind=` the census and the export, which held the two lanes between them. No clock run was asked,
so **no clock turn was taken, and the prediction is not tested by these lines; that premise of the
brief did not hold at this END**, because the batch's four other rounds were still running.
`table74.py` (ADR 1706's replay, kinds from `kind=`, kept at `/home/AI/lock-replay-1472/`) is
calibrated at 3 of 4 lines within 1 s (max 1.6 s) and gives the walks 489.2 s under the built rule,
under `--long` without the turn and under no `--long` alike (ADR 1684's rule, the census small): it
asked when lane 1 was the export's, so every two-lane rule puts it where it went. One lane: 3 783.0 s.
**Seen live, on no line yet**: from about 23:22 round 1471's campaign (`--long`, 2 400 + 2 400 +
2 700 + 3 300 s) holds lane 2, so for three hours every walk, small ones too, has lane 1 only; at
23:33 round 1470's `arms.sh` held it and four rounds' walks queued (1468's corpus, this round's check,
1470's pinned `vfs_read`, 1469's drive). That is ADR 1756 section 4's cost and no rule's: a campaign
declared small would hold a lane as long under ADR 1684. **The batch's figure is the orchestrator's to re-take** after the merge's lines:
`tools/state.sh --round <merge> gates-cost` and `cd /home/AI/lock-replay-1472 &&
PYTHONDONTWRITEBYTECODE=1 python3 table74.py batch-1468-1473`; ADR 1756 section 5 reopens the turn
if the clock runs' queue behind long holds exceeds what the turn saved the walks.

**The thread prefix.** The brief's grep printed 8 copies, not 4: its four (`doc/todo/02`'s walk line,
`doc/verify.md`'s campaign, `doc/todo/03`'s fixed-documents walk, RFC 0008's census), removed; two in
`doc/checks/`; review 1153 and trap 122, which are records. Ten more sit in crates' doc comments
outside `doc/`. `no_instruction_copies_the_threads_the_wrapper_gives_a_locked_run` in
`tools/conformance/tests/bounded.rs` now names a new copy (`RAYON_NUM_THREADS=4` before a
`bounded.sh` that takes `--lock`; another figure, an unlocked run and an assignment after `--` pass)
and holds the twelve other files, each another slot's to re-spell. **Calibrated** (trap 13): with
`doc/todo/03`'s prefix put back the test failed naming `doc/todo/03-more-corpora.md:[1054]`.
Records 1462–1467: 38 to 40 lines, each with `**Gates.**`. `doc/todo/65` re-derived before slot 1
ended: 19 `partial`, 4 `reported`, each named, no edit.

**Gates.** `bash -n` on `tools/bounded.sh`, `batch.sh`, `state.sh`: exit 0. `tools/bounded.sh
--self-test`: exit 0, every case. `cargo test -p conformance --no-fail-fast`: exit 0, 427 passed.
`RUSTFLAGS="-D warnings" cargo clippy -p conformance --all-targets`: exit 0; rustfmt `--check` on
`bounded.rs`: exit 0. `tools/batch.sh check`: exit 124 at its 1 500 s `timeout`, every static line clean, its
fuzz compile queued for lane 1 behind 1470's `arms.sh` and never run. Duration 3 600 s.
