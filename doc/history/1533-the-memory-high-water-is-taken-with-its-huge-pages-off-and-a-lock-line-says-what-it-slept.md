# 1533 — The memory high-water is taken with its huge pages off, and a lock line says what it slept

Slot 6 of batch eighty-four, 2026-10-14, an instruments round under the owner's performance target;
the partial-row word yields to it. ADRs 1902 and 1903; no question. No ledger row moves.

**Premises, held.** `bounded.sh` had no `slept`/`BOOTTIME`; `round.sh` printed "session 1528" here.
**Hypothesis (a), held in its second half**: THP is `always`, `defrag` is `madvise`. One binary,
interleaved, PDF20's first page: 19.38–20.92 MiB with the machine's pages, 17.47–18.25 with them
off (`prctl` before exec). The difference is 2–3 transient huge pages, gone by the child's end.
**What did not hold**: a decline-shaped guard. With every 2 MiB block free and no fault falling
back, children reached two huge pages or three by placement, and the two-page ones read 19.45–19.63,
under the floor. A hog holding 5.5 GiB could not drain the free blocks (compaction refilled them).

**Built (a), ADR 1902.** Every row's `peak_anon_mib` comes from one more first-page child,
`+small-pages`, whose huge pages are off (`rustix`'s safe `disable_transparent_huge_pages`, a
Linux-only dev-dependency). The gate refuses a figure whose child does not read back
`small_pages=1`. The first-page children keep the machine's pages and print theirs unbanded. All six
bands were re-derived by the file's allocation rule over twelve small-page children per row.

**Built (b), (c), ADR 1903.** `bounded.sh` reads `CLOCK_BOOTTIME − CLOCK_MONOTONIC` at the ask, the
grant and the end. Where 0.1 s or more was slept it writes `slept_queued=` and `slept=` before
`cmd=`, and `gates-cost` takes them off. `ROUND_SESSION=<n> tools/round.sh` prints the brief's
number and refuses one outside the batch. With no variable on a batch branch it prints both. All
three scripts were replaced by rename after the arms' `done` (trap 130).

**The figure moved (one clock hold, headless).** PDF20 `peak_anon_mib` before: 19.37–20.79 MiB with
blocks free, 17.76–20.79 beside builds (round 1527), floor 19.5; after: 17.38–18.14 by construction.

**Files.** `crates/viewer-ui/tests/launch_path.rs`, `crates/viewer-ui/Cargo.toml`, `Cargo.lock` (one
line), `doc/checks/launch-path.toml`, `tools/{bounded,round,state}.sh`,
`tools/conformance/tests/bounded.rs`, ADRs 1902 and 1903, this record.

**Gates.** `rustfmt --check` on both Rust files and `bash -n` on the three scripts: 0. Clippy
`-D warnings`: `viewer-ui` (export, a neighbour's `Command::Oversize` in the worktree) 0, and
`conformance` 0. `cargo nextest run -p viewer-ui` (export): 185 passed, 2 skipped. The state test
fails with `small_pages` planted as a no-op. `cargo test -p conformance`: 0, 436 passed.
`tools/bounded.sh --self-test`: 0, the slept case failing first on its own race. `tools/batch.sh
check`: 0. Clock hold (queued 281.8 s, held 355.8 s): `launch_path` exit 101, three `peak_anon_mib`
under the old floors (the method's move), bug1815476 cold open 0.507 against 0.500, 7 declined for
the carve-out. Counted gate on the new bands: 0, 31 banded, 0 outside. Duration 4 250 s.
