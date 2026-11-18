# Q907 — Shall `pdf-model`'s integration tests become one binary, a few, or stay as they are?

Source: round 2085, the pilot A876 part (2) asked for. ADR 3036 has the measurement and the layout.

## What the pilot found

`render-cpu`'s 47 integration-test files became one binary (`tests/all/main.rs`, each file a module).
The test count is the same (270 before and after), and every name is reachable as `<file>::<test>`.
Measured clean, without `sccache`, at loads of 12 to 141 on this shared machine:

- the build directory falls by 71 % (367 to 105 MiB), and a clean build's processor time by about a
  quarter; the run is unchanged, because nextest runs each test in its own process either way;
- a rebuild after one test file changes costs about three times the wall and seven times the
  processor time with incremental compilation off, as the gates build (2.3 to 6.9 s), and about the same
  with it on, as a round's edit loop may build (1.5 to 1.8 s).

## Why `pdf-model` is not settled by it

`pdf-model` has 350 test files, 138 000 lines, ten times `render-cpu`'s. The directory saving grows with
the number of files: most of Q876's 12 GiB for this crate's tests is linked binaries. But a gate's
rebuild after one test edit grows with the lines, so one binary would recompile all 138 000 lines where
it now compiles one file, every time a round edits one test and runs its gate. Seven of its files use
`#[path]` helpers, find a program from their own path, or set the environment, and need care in a
shared process under `cargo test` (not under nextest).

## The choices

1. **One binary**, as `render-cpu` now is: the largest directory and clean-build saving, the largest
   cost per test edit at the gate.
2. **A few binaries by subject** (for example content, images, fonts, forms, saving, the corpus
   walks): most of the directory saving, and an edit recompiles only its subject's binary.
3. **Leave it as it is.**

## Recommendation

**2**, done in one round that measures it the same way ADR 3036 did, with the corpus walks (`--test
corpus` and its kin, which tier 2 names) kept as binaries of their own so that their lines in
`doc/todo/02` and `tools/state.sh` do not change.

Status: open
