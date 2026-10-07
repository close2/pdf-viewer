# 1419 — The lock's descriptor stays with the wrapper, and a line with no batch is counted by its round

Slot 6 of batch sixty-five, 2026-10-07, an instruments round. ADRs 1674 and 1675. No ledger row
moved; no question written.

**Premise.** Two of its four parts held. `tools/batch.sh:188`'s `holds_the_lock` read
`/proc/$$/fd`, so `arms-held` could only prove a hold by the descriptor ADR 1659 section 4 wanted
gone; records 1408–1413 run 32–40 lines, each with `**Gates.**`. The other two did not.
"The two bare-`flock` instructions" are three lines in the named files, and the tree has twelve
outside the records: four census doc comments in `pdf-model/examples/`, `tools/main-checkout.py`'s
re-seed line, three test doc comments in the script crates and RFC 0008's command block. And the
two `batch=HEAD` lines are not the merge's own holds. They are round 1405's (`round=1405`, commands
under `scratchpad/r1405/`), written by a wrapper in a detached checkout.

**The marker.** `tools/bounded.sh` now starts the command and its `tee` with the lock's descriptor
closed. The subshell that waits for them keeps it and exports `HEAVY_WALK_HELD_BY=$BASHPID`. `--held`
accepts the marker only while it names an ancestor that has the descriptor open, and
`holds_the_lock` asks `--held`. A `--lock` run under a marked hold takes nothing and writes no line.
The output's copy moved off descriptor 3, where a bare `flock`'s lock often sits. Self-test case 8
plants the daemon, and four defects planted in copies each failed it by name: the descriptor handed
down, no marker, the marker without the parent chain, a nested run unrecognised. `batch.rs`'s new
test, replayed by hand against the first and third, saw the daemon's `arms-held` exit 0 in both.

**The instructions.** Eight lines now read `tools/bounded.sh --lock --round <session>`. A new
`bounded.rs` sweep reads 1 987 tracked text files outside the three record directories and holds
the other four files by name, on a list that may only shrink.

**The lock's cost.** `lock_cost` now places a line that names no `batch-<first>-<last>` by its
round, marks it, and finds the last batch by branch rather than by the last line. Batch
sixty-three's 1405 reads 8 runs, 4 508.4 s queued (it was 6 and 2 586.5 s). Calibrated: the old
function printed `batch HEAD` on the planted log.

**Gates.** `bash -n` on `tools/bounded.sh`, `batch.sh`, `state.sh`: exit 0 each.
`tools/bounded.sh --self-test`: exit 0, eight cases. `rustfmt --check --edition 2024` on
`tests/batch.rs` and `bounded.rs`: exit 0. `RUSTFLAGS="-D warnings" cargo clippy -p conformance
--all-targets`: exit 0. `cargo test -p conformance --test batch`: 17 passed; `--test bounded`: 5
passed. `cargo test -p conformance`: exit 0, 403 passed. `tools/batch.sh check`: exit 1, only
`cargo fmt --all --check` on siblings' files. Both scripts installed by rename after the arms
export's `done` line (trap 130). After record 1418, `the_frontier_map`: 23 open rows placed once and
no status moved, so `doc/todo/65` (slot 5's §12.10.2 bullet already true) needed no edit.
