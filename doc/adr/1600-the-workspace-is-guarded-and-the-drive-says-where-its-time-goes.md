# 1600 — The workspace is guarded at the merge's check, and the drive says where its time goes

Session 1382. Status: **accepted**. Carries out trap 114's rule as an instrument; amends ADR 1313's
`check` (two lines added) and ADR 1476's gate log (one guard). Supersedes nothing.
Context: trap 114 (`doc/traps/instruments-and-reports.md`); ADRs 1313, 1476, 1487, 1499.
Code: `tools/batch.sh` (`check_batch`, `gates`), `tools/drive-windows.sh` (`verdict`), `tools/state.sh`
(`section_drive`, `section_gates_cost`), `tools/bounded.sh` (its self-test's one Python run).
Tests: `tools/conformance/tests/batch.rs`
(`check_names_a_member_that_is_not_a_tracked_crate_and_a_pycache_under_the_globs`,
`every_python_run_in_tools_writes_no_bytecode`).

## 1. A member is a crate git tracks

A batch has one workspace for six rounds, so one directory it should not read stops every sibling's
cargo at once. Trap 114 names two ways that happens: `cargo new` under `scratchpad/` adds its crate
to the root `Cargo.toml`'s `members`, and a `__pycache__` under `tools/` is matched by the `tools/*`
glob and read as a crate with no manifest. `tools/batch.sh check` now reads `members` with
`tomllib`, expands each glob over directories the way cargo does, and names every member whose
`Cargo.toml` git does not track and every member under `scratchpad/`. A second line names any
`__pycache__` under `tools/`, `crates/` or `raster/`, whether a glob reads it or not, because it is
the leavings of the same run. **Tracked, not merely present**, because the trap's first shape has
a manifest. A round that adds a real crate is named too, until the merge stages it. That is a
finding for the merge to read, and it costs nothing.

Every Python run in a `tools/*.sh` script sets `PYTHONDONTWRITEBYTECODE=1`, on its own line or
exported once at the script's head. `drive-windows.sh` runs Python nineteen times, so it exports it.
The test reads every script and fails on a run that does neither. A comment, an `echo` and
`command -v python3` are not runs.

## 2. The summary does not overwrite the log

The last gate log began with `ALL GATES DONE`, and `build-sandbox` had no line in it. `gates()`
truncates the log, appends one line per gate, then prints the summary with `tail -1` on standard
output. If that output was redirected onto the log itself, its file offset was still nought, so
the summary landed over the first gate's line. `gates()` now compares the two inodes and prints
the summary only where standard output is somewhere else; the summary is in the log already.
`tools/state.sh gates-cost` names every gate `gates()` runs, a gate with no line as a row of its
own, and says so when an older log's first line is the summary. A `tee` onto the log cannot be
told from a terminal by its inode, and is not guarded.

## 3. A verdict carries its seconds

The drive takes about seventeen minutes and grows every batch, but `results.tsv` stated no time.
Its fifth column is now the wall-clock since the verdict before it. The first step of a window
carries that window's launch, so the column sums to the drive from the first launch to the last
verdict. It is stamped at the writer, `verdict`, because only the writer knows when a step ended.
It is in microseconds read from `EPOCHREALTIME` with the locale's radix removed. `tools/state.sh
drive` sums it per window and per step group (the step's leading number, one thing a reader does
in every window), slowest first, and names the ten slowest steps. A file from before the column is
counted as before and said to have no times. **The steps themselves are not changed here**: what
each slow step waits on is named in session 1382's record, as the reading list for a round that
changes them.
