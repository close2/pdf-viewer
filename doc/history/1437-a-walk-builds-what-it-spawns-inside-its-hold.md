# 1437 — A walk builds what it spawns inside its hold, and the self-test waits for events

Slot 6 of batch sixty-eight, 2026-10-08, an instruments round. ADR 1710; no row moved, no question.

**Premise.** Held in substance, not in its figure: of `state.sh`'s 8 `--bins` lines two are comments,
and the builds were seven commands in five places, every one before its walk queued. Every fuzz lock
line of batch sixty-seven already declared `kind=small`; the instructions said `--tree 12`. The
self-test's failure reproduces on case 7 and on case 9, both timed hand-offs. Records 1426–1431 run
36–40 lines, each with `**Gates.**`, so none needed an edit.

**Builds.** `bounded.sh --build '<args>'` runs `cargo build` inside the hold and the bound, lanes
closed, before the command; a failed build ends the run unstarted and `cmd=` stays the gate's.
`state.sh`'s `walk` derives the sandbox worker of its command's profile (`--walk-worker` prints it),
the oracle and the launch path declare `pdfref-hayro` and `pdf-script-worker`, and `gate_binaries` is
gone. The vfs and confined builds were dropped: Cargo builds a package's own bins for its tests (the
unit graph lists `pdf-view-worker`). Three `bounded.rs` tests hold it, each calibrated by plants;
self-test case 10 runs a stand-in `cargo`, and three planted defects each failed it. Live: `state.sh
--round 1437 dates` exit 0, its line naming the gate; a warm worker build held 1.2 s.

**Fuzz.** Peaks from the log: seeding and `check` 0.01–1.29 GiB for ten targets, 5.34–5.55 GiB for
`jbig2`/`jpx`; so `--tree 6`, and `--tree 12` for those two, in `seeds.sh`'s header and
`doc/verify.md`, where the fuzz build now goes behind the lock too (1.37–1.65 GiB). Campaigns stay
small walks (0.04–0.50 GiB); round 1428's six clock runs waited 2 281.6 s behind them, stated in the
ADR. `batch.sh check` runs `cargo check` over `fuzz/Cargo.toml` behind the lock: `clean` here, and a
sandbox test plants a type error (`fails`, `error[E0308]`) and no manifest (`none`).

**Self-test.** Holds release on the run's "queued" line, waits are bounded at 60 s, "waited nothing"
is under one 0.5 s poll, case 1 bounds processor time. Pinned to two processors the old script failed
2 of 2 at six busy loops and 1 of 1 at two; the new one held 4 of 4 at 6, 6, 12 and 24 (queues 4–13).

**`doc/todo/65`**, re-derived after slot 5 from the 23 open rows' notes: no status moved and no row
changes bucket; bucket 6 now says §7.4.9's non-D50 Lab case is buildable. **Unfinished**: the merge's
own build lines stay (it runs alone); traps 10 and 109 still spell a bare `cargo build … --bins`.

**Gates.** `rustfmt --check --edition 2024` on `bounded.rs`, `read_only.rs`, `batch.rs`: exit 0.
`RUSTFLAGS="-D warnings" cargo clippy -p conformance --all-targets`: exit 0. `cargo nextest run -p
conformance`: 412 passed, 0 failed. `cargo test -p conformance`: exit 0. `bash -n` on `bounded.sh`,
`batch.sh`, `state.sh`, `fuzz/seeds.sh`: exit 0 each. `tools/bounded.sh --self-test`: exit 0, every
case holds. `tools/batch.sh check`: exit 1, on `cargo fmt --all --check` alone, in a sibling's file;
the fuzz line `clean`. The three scripts installed by rename after the arms' `done` line (trap 130).
