# 2085 — `render-cpu`'s integration tests are one binary

Session 2085, a measure round in `pdf-viewer-audit`: the owner's A876, both parts.

- **Part 1.** `doc/todo/02` section 0's tier 1 now runs `cargo nextest run -p <crate> --lib --bins --tests`
  and says why: `clippy --all-targets` type-checks the examples in the round, and the post-merge train
  links them.
- **Part 2, the pilot.** The 47 files of `crates/render-cpu/tests/` are modules of `tests/all/main.rs`,
  moved unedited (the four owner audit files among them, A868/A875). No file had a `#[path]` helper, a
  program lookup from its own path, an environment or working-directory change, or a `static`, so ADR
  2914's rule was not reached. **Test count: 270 before, 270 after** (82 library, 188 integration), the
  sorted name lists identical once `all::` is stripped.
- **Names**: nextest's `render-cpu::<file>` becomes `render-cpu::all` with tests `<file>::<test>`; `--test
  <file>` becomes `--test all <file>::`. **A new test file joins** by going into `tests/all/` with a `mod`
  line in `main.rs` (round 2083's, if it adds one, at the merge); a file left in `tests/` is its own binary.
- **ADR 3036** holds the table (clean builds without `sccache`, three in sequence and three pairs
  interleaved, load beside each): the directory 367 → 105 MiB, a clean build's processor time about a
  quarter less, the run unchanged, a one-file rebuild 2.3 → 6.9 s with incremental off and 1.5 → 1.8 s
  with it on. **Q907** asks the owner about `pdf-model` (one binary, a few by subject, or none),
  recommending a few by subject.
- The 27 ledger rows citing these tests name `tests/all/<file>.rs`; no status changed. Live path references
  in other crates' comments and in `doc/` follow; records (`doc/reviews`, `doc/adr`, `doc/history`,
  `doc/questions`) are left as written. No tool, CI line or `audit-diff.py` input named a `render-cpu`
  test binary; `expected-failures-current.json` has no `render-cpu` entry.

**Gates.** Over `-p render-cpu -p pdf-render -p pdf-model -p render-gpu -p render-raster` (the last four
for comment paths only): fmt `--check` exit 0; clippy `-D warnings --all-targets` exit 0; `pdf-sandbox --bins`
exit 0; nextest `--lib --bins --tests` of `render-cpu`, `pdf-render` and `pdf-model` under `xvfb-run -a` exit 0,
3453 run, 3453 passed, so the manifest's difference is empty (it lists none of the three); `render-gpu` and
`render-raster` were not run, their change being two doc comments in test files; rustdoc `-D warnings --document-private-items` of the five exit 0;
conformance 2 failed, both this record before this paragraph, then 0. Fuzz not owed: no public type changed.

Trap: on this machine a "clean build" is a cache read, because the user's cargo configuration wraps
`rustc` in `sccache`; `RUSTC_WRAPPER=""` turns it off for the measured build only.
