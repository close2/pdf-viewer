# 1485 — A batch's lock lines are re-taken by one command, and the arms export says when its hold ended

Slot 6 of batch seventy-six, 2026-10-09, an instruments round. ADR 1806. No ledger row: an instrument.

**Premise, held.** `grep -c 'batch=batch-1474-1479' /home/AI/heavy-walk.log` prints 50 (13 rounds',
the export, 3 `check`, the merge's 33). The replay takes 17: 15 walks, 0 clock runs, 1 long hold, the
export. Batch seventy-four's figures re-ran unchanged through the kept `table74.py`.

**Hypothesis, half held.** No clock run, as predicted: ADR 1756 section 5's C = 0.0 s against S =
0.0 s, so the turn stands untested. **The longest queue was not behind the batch's export**: round
1475's drive queued 930.7 s behind round 1478's own six-arm run (1 549.2 s on lane 1). The one long
hold, 1478's census as `--long`, was killed at lane 2's 6 GiB (peak 6.77 GiB, exit 137) after 828.5 s
and ran again as a large walk after 844.8 s of queue.

**ADR 1790 section 6.** One pair out of ask order: 1474's walk asked 01:56:45 was granted lane 1 at
02:02:48 while 1475's, asked 01:53:41, waited to 02:04:16 — both wrappers older than the rename, so
the race. Walks queued 2 774.6 s observed, 2 773.8 s in the race replay (17/17 within 1 s, summed
error 2.8 s): the order stands. Ask order replays to 2 802.6 s: the pair's 998.6 s becomes 940.4 s,
and the chain asks 1475's drive 88 s earlier behind the same release (930.7 → 1 018.0 s, same grant).

**The command.** `tools/state.sh gates-cost` reads `LOCK_COST_BATCH`'s batch (or the last), names
each pair granted out of ask order, and prints `cd /home/AI/lock-replay-1485 && PYTHONDONTWRITEBYTECODE=1
python3 retake.py <batch>`; `retake.py` prints section 5's C, S and verdict and section 6's pairs and
queue against the race. **Calibrated** (trap 13): both name 11 pairs on batch seventy-four's lines and
1 on seventy-five's; none on the ask-order replay's own grants of four batches, 2/0/10/1 on the race
replay's; with the lane test removed the planted control was named. **The export's hold**: `arms`
ends the README with `released <time>, the lock's line: <wrapper's line to cmd=>`; against HEAD's
script the README ended at `build exit 1` and the new test failed.

**Files.** `tools/state.sh` (`lock_cost`) and `tools/batch.sh` (`export_arms`, `arms_released`), each
renamed in after the export's `done` at 04:30 (trap 130); `tools/conformance/tests/bounded.rs` and
`batch.rs` (a test each); ADR 1806; this record. Outside the tree: `/home/AI/lock-replay-1485/`.

**Gates.** `bash -n` on `bounded.sh`, `state.sh`, `batch.sh`: exit 0. `tools/bounded.sh --self-test`:
exit 0, every case, 61 s. `cargo test -p conformance --no-fail-fast`: exit 101, both new tests
passing; the two failures are siblings' mid-edit lines (`round_numbers` on slot 5's `serve.rs:185`
knob, `conformance` on slot 1's `submission.rs:345` and `view.rs:3164`). `clippy -p conformance --all-targets` (`-D warnings`) and
`rustfmt --check` on `bounded.rs`, `batch.rs`: exit 0. `tools/batch.sh check`: exit 124 at its 1 500 s
`timeout`, every static line clean, its fuzz compile queued 1 499.6 s behind round 1482's and 1484's
clock runs and written as `lane=-` (ADR 1790's stopped line, live). Duration 3 470 s.
