# 1479 — The script worker's tests hold the question's clock and wait on the worker's own death

Slot 6 of batch seventy-five, 2026-10-09, a script round. ADR 1794; ADR 1795 and Q375 not used. No
ledger row moved: the contract is a test and no clause is reached.

**Premise.** Mostly held. The lines stood as quoted: 339, 595, 642, 608, 659. Line 339 is not a race,
since its 50 ms deadline is a kill the test chooses on a script that cannot finish. The races were in
two tests the brief did not name. The growth test was killed by the 250 ms watchdog in 2 of 20 runs
on one core shared with eight busy loops. The filter test failed in 8 of 80 runs at a load average
of five to six, every time on a 20 000-element `sort` that took 138 to 152 ms when it lived. Plain
`cargo nextest run -p pdf-script-worker` runs only the 4 wire tests, since `end_to_end` needs
`--features engine`.

**Hypothesis.** Held. Withdrawal is decided in the host's half (`expire`), so `wire::VERSION` is
untouched.

**Built.** `HeldClock` and `ScriptWorker::with_clock`. The two withdrawal tests run at the real
`ANSWER_WAIT`: nothing is withdrawn one nanosecond short of the wait, and it is withdrawn at the wait.
`expire` now withdraws at `>=` the deadline, which agrees with `question_deadline`. The growth and
filter tests run under `OUT_OF_THE_WAY` (1 min), so their verdict is the worker's own signal. ADR 1794
names the four bounds that stay timed, each of which passed 20 of 20 runs on the contended core.

**The rest of the sleeps** (13 lines in 9 files, down from 15 in 10, none read): `pdf-model`
`save_round_trip.rs:391`, `text_extraction.rs:215`; `pdf-sandbox` `in_process_deadline.rs:97`,
`reaping.rs:91`; `pdf-transform` `foreign_corpus.rs:484`; `render-raster` `turn_path.rs:260,611,908`;
`viewer-host` `signature_policy.rs:253,286`, `fetch_import.rs:146`, `submit.rs:350`; `viewer-core`
`headless.rs:8776`.

**Gates.** `rustfmt --check --edition 2024` on `client.rs`, `lib.rs` and `end_to_end.rs`: exit 0.
`cargo clippy -D warnings -p pdf-script-worker --all-targets`, with and without `--features engine`:
exit 0 each. `cargo nextest run -p pdf-script-worker`: exit 0, 4 passed. With `--features engine`,
ten runs in a row: exit 0 each, 26 passed and 1 skipped each. `cargo check -p viewer-host`: exit 0.
Plants (trap 13): unheld (HEAD's test file), the withdrawal tests averaged 310 ms against 4 and 5 ms
held; the filter test failed 8 of 80 against 0 of 80; the growth test failed 2 of 20 against 0 of 20.
With `expire` reading `Instant::now`, both held tests failed, and with the boundary back at `>`, both
failed. `cargo test -p conformance --no-fail-fast`: 427 passed and 1 failed, which is
`the_lock_cost_lists_each_wait_that_ended_before_a_grant`, a test only a sibling's uncommitted
`bounded.rs` holds. Tier 2 was not run, since no corpus walk waits out a question or races the
watchdog. Duration 1 950 s.
