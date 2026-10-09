# 1477 — A walk is granted in the order it asked, and a wait stopped before its grant is a line

Slot 4 of batch seventy-five, 2026-10-09, an instruments round. ADR 1790. No ledger row: an instrument.

**Premise, held.** On `batch=batch-1468-1473` round 1470's small `read_corpus`, asked 23:01:16, queued
4 339.7 s; round 1468's `raster_golden`, asked 23:12:20, 490.8 s, and its `corpus`, 23:21:38,
3 056.4 s, `behind=` the same census, campaign and export. `lock_take`'s walk branch was `flock -n` per
lane in a `sleep` loop with no order. Round 1469 has two lines, both after its stopped request, and
round 1472's `batch.sh check`, killed by its 1 500 s `timeout` while queued, has none. **Found beside
it:** a clock run waits in a foreground `flock`, which holds back every trap, so `TERM` there ended the
run only at its grant, on no line.

**Hypothesis, held.** A ticket file `<lock>.queue` (pid, start time, lanes, holder word), read and
written under its own `flock` for one look and one grant, orders every walk; a walk takes lane N only
where no live earlier ticket takes N, so a small walk behind a large one's ask keeps the second lane.
Clock runs take no ticket; the turn, the gate and `--long` are unchanged. Dead, zombie and reused-pid
tickets drop at the next look, a stopped process's holds back nobody, and a look past 2 s of the
queue's lock is retried rather than waited for. A stopped wait writes `hold=0.0s … lane=-`; a clock
run's `flock`s run under `wait`. `gates-cost` marks, lists and sums the stopped runs.

**Priced.** `table74.py` (kept at `/home/AI/lock-replay-1472/`, unchanged) on batch seventy-four's 19
lines, calibrated 15 of 19 within 1 s: the pollers' race gives the 15 walks 11 125.2 s, max 4 293.0 s;
ask order 10 202.5 s, max 1 932.7 s, median 45.3 → 210.8 s.

**Calibrated** (trap 13), each plant failing its case by name: the order look emptied (case 13, walk
135 granted before 133), the stopped line removed, the clock `flock` in the foreground (the line only
after the holder ended), the dead-ticket drop and the stopped-process skip removed (case 14, the walk
after them never granted). `the_lock_cost_lists_each_wait_that_ended_before_a_grant` against the old
`state.sh`: no stopped line listed. The new self-test passed three times side by side.

**Files.** `tools/bounded.sh` and `tools/state.sh` (`lock_cost`), each renamed in after the arms export's
`done` (trap 130); `tools/conformance/tests/bounded.rs` (a test, the module comment); `doc/environment.md`'s
rule line; `doc/todo/02` section 0 item 3; ADR 1790; this record.

**Gates.** `bash -n` on `bounded.sh`, `state.sh`, `batch.sh`: exit 0. `tools/bounded.sh --self-test`:
exit 0, every case, 70 s. `cargo test -p conformance --no-fail-fast`: exit 0, 428 passed; again with
this record, exit 101, 427 passed, the one failure slot 1's record 1474 at 41 lines. `clippy -p
conformance --all-targets` (`-D warnings`) and `rustfmt --check` on `bounded.rs`: exit 0. `tools/batch.sh
check`: exit 0, every line clean, its fuzz compile a small walk on the new wrapper (wait 0.5 s, the live
queue file empty after). Duration 2 640 s.
