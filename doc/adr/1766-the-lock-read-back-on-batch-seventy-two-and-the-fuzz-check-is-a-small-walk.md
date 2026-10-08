# 1766 — The lock read back on batch seventy-two, and the fuzz workspace's check is a small walk

Session 1466. Status: **accepted** and **built**. Reads back ADR 1756's prediction and ADR 1706's
re-ask on the first batch after `--long` landed; amends ADR 1710 section 3 (the kind of `tools/batch.sh
check`'s compile) and the thread share `tools/bounded.sh` gives a locked run. Supersedes nothing.
Context: ADRs 0798, 1684, 1698, 1706, 1710, 1718, 1756; traps 13, 122, 130; habit 76.
Code: `tools/batch.sh` (`check`), `tools/bounded.sh` (`locked_threads`), `doc/environment.md`'s rule
line. Tests: `tools/conformance/tests/batch.rs`
(`check_compiles_the_fuzz_workspace_and_names_a_target_that_does_not`),
`tools/conformance/tests/bounded.rs` (`a_locked_run_walks_at_the_merges_four_threads`). Instrument:
ADR 1756's replay, with `check72.py` beside it, kept at `/home/AI/lock-replay-1466/`.

## 1. Batch seventy-two, read back

`tools/state.sh --round 1466 gates-cost` over the 73 lines that say `batch=batch-1456-1461`: the
rounds' 36 lines queued **5 885.0 s** (1461 2 946.4 s, 1459 1 544.4 s, 1458 1 206.9 s, 1457 187.3 s),
the merge's 33 and the arms export 0 s, and the three `round=check` lines **4 659.8 s for 6.6 s of
hold** — 44% of the batch's 10 544.8 s. **No line says `kind=long`**: the kind landed at 18:13 and
the batch ran one census-shaped run after it, round 1457's 23.2 s `fuzz/seeds.sh … script_wire`,
as a small walk. So no clock turn was taken and no run queued behind a long hold.

**The prediction is not tested by this batch, and that premise of the brief did not hold.** ADR 1756
section 3 priced batch seventy-one's shape: nine campaign and census holds, 14 170.8 s. Replayed with
ADR 1756's labels, batch seventy-two relabels one line, and the built rule, `--long` alone and `--long`
with the turn each give the rounds 5 862.4 s (against 5 885.0 s observed, 31 of 37 lines within 1 s):
there was nothing in the second lane for the rule to move. ADR 1706's re-ask holds on this batch too:
one lane less the built rule is 6 944.1 s, above its 3 600 s.

## 2. What the queue was: a compile on the first lane

The rounds' five waits over 600 s are each a large walk or a clock run behind another: 1461's
`arms.sh` behind the export (655.4 s), 1459's behind 1461's (1 233.8 s), 1461's two `turn_path` runs
behind 1459's `arms.sh` and 1458's drive (1 244.4 and 842.8 s), the drive behind the first of them
(1 206.9 s). Those are the rule working. The check's two are not: `cargo check -q --manifest-path
fuzz/Cargo.toml` is a build and ADR 1710 declared it a large walk, for a cold build of the fuzz
workspace's dependencies. It then waited for the first lane behind the export and behind a clock run,
and lost the pollers' race to walks asked after it: the replay in ask order gives the first 858.2 s
where it waited 2 411.5 s. A third, round 1460's, ran under `timeout 900` and died queued, on no line.

**Its fifteen lines on the log peak at 0.02 to 1.58 GiB and hold 1.1 to 29.8 s**, the cold ones
included (0.13, 0.78, 0.91, 1.58 GiB). That is inside the second lane's 6 GiB with room.

## 3. Decision

`tools/batch.sh check` runs the fuzz workspace's compile as a small walk, `--tree 6`, on the second
lane or the first, as habit 76 says a build runs. Replayed on batch seventy-two the check's queue
falls from 2 168.8 s (in ask order) to 1 307.8 s, the first to 1.6 s; the second still waits 1 306.2 s
for a clock run, which is right, because a compile beside a clock run moves its verdict. The rounds'
replayed queue does not rise (5 877.8 to 5 862.4 s). **Calibrated** (trap 13): the test's new
assertion that the line says `kind=small lane=2` failed on the old script with `kind=large lane=1`.

## 4. A locked run's threads are the wrapper's four (from slot 1, round 1462)

The rule line wrote `RAYON_NUM_THREADS=4 tools/bounded.sh --lock …`, and the wrapper exported
`nproc / --shards` — 24 here — to a caller that had not set the variable, so a locked run's thread
count was whatever its round remembered to type. **The four is right** and the wrapper now owns it: a
walk's peak is its documents in flight, one a thread (ADR 0798), the lanes' ceilings and the kinds
were measured at the merge's four (ADR 1698), and `tools/batch.sh`'s gates, the arms export and
`tools/state.sh`'s walks already set it. A run under `--lock` gets four threads whatever `--shards`
says; a run outside the lock keeps `nproc / --shards`; a caller's own setting is kept, which is how a
clock child pins its own (trap 122). The rule line drops the prefix and says so. **Calibrated**:
`a_locked_run_walks_at_the_merges_four_threads` reads 24 from the wrapper before this change.

## 5. Re-ask

ADR 1756 section 5's re-ask stands and is now owed by the first batch that runs a `--long` hold:
`tools/state.sh gates-cost` lists each, and `table71.py <batch>` and `long71.py <batch>` in the kept
directory replay it. If a cold check ever peaks above 5 GiB, it returns to the first lane.
