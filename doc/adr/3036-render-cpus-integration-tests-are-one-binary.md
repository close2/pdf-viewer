# 3036 — `render-cpu`'s integration tests are one binary

Session 2085. Status: **accepted for `render-cpu`; `pdf-model` is the owner's, Q907.** It cites no clause.
It moves no ledger row's status; the 27 rows whose `test` sites name a `render-cpu` integration test now
name `crates/render-cpu/tests/all/<file>.rs::<test>`. Context: A876 part (2), Q876 §2, ADR 2912 (linked
programs are most of a build directory), ADR 2914 (a program found from a test's path).

## 1. What moved

The 47 files in `crates/render-cpu/tests/` are now modules of `tests/all/main.rs`, which Cargo finds
as the test target `all` without a `[[test]]` entry. No file was edited: each one's `//!` header and its
`#![expect(...)]` block are a module's inner attributes as they were a crate's. None of them had a
`#[path]` helper, a program lookup from its own path, a `set_var`, a working-directory change or a
`static`, so nothing was shared to merge and nothing needed ADR 2914's rule. The owner's four audit files
(`*_spec_audit.rs`, `*_audit.rs`) moved as modules, unedited (A868, A875).

**The count is equal**: `cargo nextest list -p render-cpu` names 270 tests before and 270 after, 82 of
the library and 188 integration tests, and the sorted lists are identical once `all::` is stripped.

**Names and filters.** nextest's binary id `render-cpu::<file>` becomes `render-cpu::all`, and the test
`<test>` becomes `<file>::<test>`. So `--test <file>` becomes `--test all <file>::` under `cargo test`
and `-E 'binary(all) & test(/^<file>::/)'` under nextest; a ledger site is a path and names the file
under `all/`. **A new test file joins** by being placed in `tests/all/` and named by a `mod` line in
`main.rs`; a file left in `tests/` is a binary of its own again, and `cargo test` warns of nothing.

## 2. The numbers

`CARGO_INCREMENTAL=0`, `RUSTC_WRAPPER=""` (no `sccache`, so a clean build is a clean build), a private
target directory removed before each build, `cargo nextest run -p render-cpu --no-run` under
`tools/bounded.sh --tree 24`; then `cargo nextest run -p render-cpu`; then the same build after touching
`stroke_width.rs`. Processor time is user plus system of the children. Two sittings: three of each in
sequence, then three pairs interleaved (habit: A/B in one sitting). Load is the one-minute average
before the build. Four lanes and another user were building throughout.

| | load | build wall s | build CPU s | directory MiB | test binaries | run wall s | one-file rebuild wall / CPU s |
|---|---|---|---|---|---|---|---|
| before, sequence | 28, 36, 46 | 22.4, 25.0, 18.1 | 171, 174, 159 | 367 | 47 | 25.3, 24.5, 30.8 | 2.3 / 2.8, 2.3 / 2.7, 2.3 / 2.7 |
| after, sequence | 12, 22, 22 | 15.6, 16.7, 25.2 | 105, 114, 154 | 105 | 1 | 32.6, 24.9, 28.8 | 7.1 / 24.7, 6.7 / 18.5, 6.7 / 19.1 |
| before, interleaved | 27, 41, 87 | 25.0, 56.1, 19.1 | 179, 220, 162 | 367 | 47 | 34.1, 50.5, 24.0 | 3.6 / 3.7, 18.5 / 8.3, 2.3 / 2.6 |
| after, interleaved | 33, 141, 70 | 23.1, 181.9, 17.4 | 141, 222, 118 | 105 | 1 | 35.4, 26.7, 29.2 | 9.0 / 23.8, 8.1 / 21.9, 4.5 / 15.1 |

With incremental compilation on (`CARGO_INCREMENTAL=1`, the edit loop a round may use), the same
touched-file rebuild was 1.55, 1.58 and 1.50 s before and 1.81, 1.76 and 1.82 s after, at a load of 35 to
39: the query cache keeps the 46 modules that did not change. A touch changes no content, so an edit that
does recompiles its own module's code as well; that part is the same under either layout.

What the numbers say, medians over the six of each:

- **The directory falls by 71 %**, 367 to 105 MiB, at every load: 46 linked copies of the crate's graph
  are gone. This is the figure ADR 2912 predicted and Q876 could not measure.
- **A clean build's processor time falls by about a quarter** (173 to 129 s by median); its wall time
  falls a little (24 to 20 s), because the dependencies, not the tests, are its critical path.
- **The run is unchanged within the noise** (28 to 29 s): nextest runs each test in a process of its own
  under either layout, and the run is held by one long test (`thin_segment_ladder`, 22 s).
- **A one-file rebuild costs about three times the wall and seven times the processor time** (2.3 to
  6.9 s, 2.8 to 20.5 s): one edited test recompiles all 13 000 lines of the 47 modules, where it
  recompiled one file before.

## 3. What it costs

- **The whole binary rebuilds on any test change** under the gates' `CARGO_INCREMENTAL=0` (§2's last
  column); in an incremental edit loop the difference is a quarter of a second (§2).
- **Failure isolation is unchanged under nextest**, which spawns a process per test whatever the
  binary; under `cargo test` a test that aborts the process (a stack overflow, an `abort`) now takes the
  other 187 with it, where before it took its own file's. nextest is what the gates run.
- **Parallelism is unchanged under nextest**, for the same reason. Under `cargo test` it improves: one
  binary's harness schedules all 188 tests over its threads, where 47 binaries ran one after another.
- **A compile error in one module stops all of them**; before, the other 46 binaries still built. Clippy
  `--all-targets` reports every error either way.

## 4. Recommendation for `pdf-model`

Merge `render-cpu` as built here. It is a small crate whose tests change rarely; the directory and the
clean build win, and the one-file rebuild costs seconds.

`pdf-model` is a different trade, and the owner's: its 350 test files are 138 000 lines, ten times
`render-cpu`'s, beside eight subdirectories of fixtures and shared modules, and seven of them use `#[path]`, find
a program from their own path, or set the environment.
The directory saving scales with the files (most of Q876's 12 GiB), but the one-file rebuild scales with
the lines, so a round that edits one `pdf-model` test would recompile all of them. Q907 asks between
one binary, a few binaries by subject, and leaving it.
