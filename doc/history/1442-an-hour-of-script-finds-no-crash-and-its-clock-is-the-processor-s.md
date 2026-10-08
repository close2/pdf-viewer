# 1442 — An hour of `script` finds no crash, and its clock is the processor's

Slot 5 of batch sixty-nine, 2026-10-08, a fuzz round. ADRs 1716 and 1717; no row moved, no question.

**Premise.** Held: ADR 1694's table has `script` 13 479 → 20 819 and `vfs_write` 5 968 → 8 401 in
600 s, both still finding; `doc/verify.md`'s lines give the limits used. The census before the
campaigns found all five targets current.

**Campaigns.** `script` and `vfs_write` 3 600 s, `aform`, `forms_data` and `embed` 1 800 s, all
five side by side in one `--tree 6` hold (3 611 s, peak 0.53 GiB), fork mode past every stop,
scratch corpus first. **No crash, timeout or memory refusal on any**, so no regression test and
nothing under `fuzz/artifacts/`. Edges after, a `-runs=0` pass over disk corpus plus finds:
`script` 13 515 → 20 170, `vfs_write` 5 968 → 8 306, `aform` 1 369 → 2 062, `forms_data`
1 387 → 2 558, `embed` 1 257 → 2 653 (ADR 1716 has executions and the last tenth's gain).

**Found by the census after.** `fuzz/seeds.sh check script` stopped as "not judged": two seeds at
the element budget took 2.66 s and 2.92 s of wall time at nice 19 under load, against 0.42 s each
at rest. The target's escape property now reads the thread's processor time from
`/proc/thread-self/schedstat` and prints wall time and run-queue wait beside it (ADR 1717);
planted at 100 ms it names the seed at 428 ms on a processor, 430 ms wall. `script_peak` in release:
the 2^20 `fill` 135.0 ms, the typed array's `join` 352.7 ms — ADR 1609's known class.

**Wire version.** Slot 1 raised `wire::VERSION` to 9; `script_wire` was re-seeded after it (32
seeds, names overwritten) and its census reads 710 / 710. The re-seed followed version 9.

**Unfinished.** The campaigns' finds stay in `scratchpad/r1442/run/` (ADR 1423), not merged.
`fuzz/Cargo.lock` moved with slot 4's hayro pins when the fuzz workspace resolved; it is slot 4's.

**Gates.** `rustfmt --check --edition 2024 fuzz/fuzz_targets/script.rs`: exit 0. `cargo fmt
--manifest-path fuzz/Cargo.toml --check`: exit 0. `RUSTFLAGS="-D warnings" cargo clippy
--manifest-path fuzz/Cargo.toml --all-targets`: exit 0. `cargo check --manifest-path
fuzz/Cargo.toml`: exit 0. `cargo test -p conformance --test fuzz_workspace`: 4 passed. `cargo test
-p conformance`: 417 passed, 0 failed. `fuzz/seeds.sh check`: before, 5 current; after, 4 current
and `script` not judged; after ADR 1717, `script` current 13 535 / 13 533, `script_wire` current.
Five campaigns, `tools/bounded.sh` exit 0, libFuzzer exit 0 each. Six holds of the lock.
