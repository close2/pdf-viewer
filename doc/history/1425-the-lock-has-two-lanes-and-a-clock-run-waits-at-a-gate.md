# 1425 — The lock has two lanes, and a clock run waits at a gate

Slot 6 of batch sixty-six, 2026-10-08, an instruments round. ADR 1684; no row moved, no question.

**Premise.** It held. ADR 1671 section 4 decided the lane and left it owed; `--self-test` had eight
cases; `lanes.py` on batch sixty-four's 64 lines gives 5 443.7 and 7 978.6 s, the model's two rows.
Records 1414–1419 run 36–40 lines, each with `**Gates.**`, so none needed an edit.

**The lanes.** `tools/bounded.sh --lock` grants on the declared kind: `--tree 6` or less takes the
second lane, then the first; a larger walk takes the first; `--clock` takes both. A clock run holds
`<lock>.gate` while it waits, and walks poll gate and lanes twice a second, so a waiting clock run
stops every grant. A `--clock` inside a one-lane hold, or without `--lock`, exits 64. The line gains
`kind=` and `lane=` before `cmd=`; `lock_cost` prints the lane. Self-test case 9 plants a clock run
behind two lane holders and a small walk asked after it. Five plants each failed the self-test by
name: pollers ignoring the gate, a clock taking one lane, a small walk confined to the first, the
command keeping the lanes, a nested clock accepted. Replayed by hand, `batch.rs`'s daemon check
saw the fourth plant's daemon keep lane 2, which its one-lane question had called free.

**The callers.** `tools/batch.sh`'s `run` passes `--tree 12` and, for the six names in
`clock_gates`, `--clock`; the arms export stays large (it peaked at 4.50 GiB here, builds inside).
The rule line and `doc/todo/02` section 0 item 3 say the kinds. Two new `bounded.rs` tests: every
`--lock` in 1 987 tracked text files declares a kind (`substitution_census.rs` held, not mine), and
every clock gate is a gate `gates()` runs. Both failed on the old `run`.

**The replay.** `scratchpad/r1425/replay/built.py` steps sim.py's chains over two lanes. Under the
model's rule it reproduces both `lanes.py` rows exactly; on one lane it is 33.2 s above sim.py's
14 363.3 s. **The built rule gives 4 788.6 s, 9 574.7 s under one lane**, which beats the model's
6 385 to 8 920 s because batch sixty-four's eleven clock runs (1.1 to 232.4 s) now pass the walks
queued before them. Declaring small only what peaked under 4 GiB gives 7 189.6 s.

**Unfinished.** Wrappers queued before the install block on lane 1 and ignore the gate until
granted. `tools/state.sh`'s corpus sections walk through the wrapper without `--lock`.

**Gates.** `bash -n` on `tools/bounded.sh`, `batch.sh`, `state.sh`: exit 0 each. `--self-test`:
exit 0, nine cases. `rustfmt --check --edition 2024` on `tests/bounded.rs`, `batch.rs`: exit 0.
`RUSTFLAGS="-D warnings" cargo clippy -p conformance --all-targets`: exit 0. `--test bounded`: 7
passed; `--test batch`: 17. `cargo test -p conformance`: 405 passed; after record 1424, 404 and 1
failed, `names.rs` on a sibling's `raster-gpu` hunk. `tools/batch.sh check`: exit 0. Scripts
installed by rename after the arms' `done` line (trap 130). `the_frontier_map` passed after 1424;
`doc/todo/65`'s rich text heading gained ADR 1682, its §12.7.4.3 bullet already true.
