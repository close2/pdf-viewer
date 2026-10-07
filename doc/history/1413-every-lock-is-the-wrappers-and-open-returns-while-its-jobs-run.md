# 1413 — Every lock a tool takes is the wrapper's, and `open` returns while its jobs run

Slot 6 of batch sixty-four, 2026-10-07, an instruments round. ADRs 1662 and 1663. No ledger row
moved; no question written.

**Premise.** It held: `tools/batch.sh:241` took the lock with a bare `flock`, and so did line 171,
the arms export, which the brief did not name; `doc/environment.md:13` spelled it `flock …`;
`tests/bounded.rs` said "six cases" of seven. Records 1402–1407 run 36–39 lines, each with
`**Gates.**`, and `records.rs` holds that, so nothing was owed there.

**`open` in the foreground.** The jobs were started as `(cd "$wt" && setsid nohup … &)`, and `&`
binds looser than `&&`: a copy of the shell ran the job in its foreground and held the caller's
pipe. This batch's copy, pid 2859031, sat in `do_wait` with descriptors 1 and 2 on that pipe until
the export ended at 17:52, 1 659 s after its lock was taken. One `detach` now starts both jobs with
`cd` and `;` before them, the warm build first and the export beside it (ADR 1662 gives the
reason). A 6 s job: 6 012 ms the old way, 3 ms the new. The new test uses a 60 s stand-in `cargo`.
With the old construction planted back, its `open` took 122.7 s and the test failed.

**Every lock through the wrapper.** `run` takes the lock with `tools/bounded.sh --lock --round
<branch> --nice 0`. It reads `wall` from the wrapper's `after <n>s` and `wait` as the rest; a
harness against a held scratch lock logged `wait=3s`, and the wrapper's line said 2.7 s. `arms`
runs `arms-held` under `--lock --round arms`, which refuses without the lock. The sandbox test sees
its hold logged as `round=arms`. The rule line now spells `--lock --round <session>`. A new sweep in
`tests/bounded.rs` fails any `flock` in `tools/*.sh` other than the wrapper's. Before this change it
named both lines.

**The trap index.** The reader split cells on every `|` and dropped a five-cell row without a word.
It now splits at unescaped bars and names a malformed row by its line (ADR 1663). Calibrated: a
bare `| tail` in trap 127's row was named at line 176, and the gate passed again once it was removed.

**The frontier map.** Re-read after slot 5: its three rows stay `partial`, and `the_frontier_map`
places 23 open rows once each. Bucket 4's heading said "four rows" over three, so it names no count.

**Gates.** `bash -n` on `tools/batch.sh` and `tools/state.sh`: exit 0 each; rustfmt `--check` on
`tests/batch.rs`, `bounded.rs` and `traps.rs`: exit 0; `RUSTFLAGS="-D warnings" cargo clippy -p
conformance --all-targets`: exit 0; `cargo test -p conformance --test batch`: 16 passed, `--test
bounded`: 3 passed, `--test traps`: 2 passed; `cargo test -p conformance`: exit 101, 43 binaries
ok and `records.rs` failing only on 1409's and 1412's placeholder `**Gates.**` paragraphs, which
this record passes; `tools/batch.sh check`: exit 1 on the same two, `cargo fmt --all --check` clean.
