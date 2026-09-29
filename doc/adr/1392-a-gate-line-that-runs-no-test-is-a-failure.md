# 1392 — A gate line that runs no test is a failure, and every line's flag is held to its file

Session 1277. Status: accepted and **built**.
Context: `tools/batch.sh` (`run`), `tools/conformance/tests/batch.rs`
(`every_gate_line_runs_at_least_one_test_of_the_file_it_names`, `test_shape`, `gate_commands`),
`doc/todo/02-every-round.md` section 2's change-to-gate map.
Builds on: ADR 1036 (tiers 2 and 3 run once per batch), ADR 1313 (`tools/batch.sh`).

## 1. What was green and checked nothing

`cargo test --test X -- --ignored` runs only the ignored tests of `X`; over a file with none it
prints `0 passed` and exits 0. `doc/todo/02`'s map gave `-p pdf-transform --test gate` with no
`--ignored`, and `gate.rs`'s one test is `#[ignore]`d: the line ran zero tests. `render-gpu`'s row
named `headless_gpu` with no command, and a round ran it with `--ignored` — zero of its tests are
ignored, so that too ran nothing while the same command without the flag runs every one. Both
lines of `doc/todo/02` now carry the flag their files need, and `pdf-archive`'s map line gained the
`--ignored` its walk needs (without it, it ran the file's one unit test and not the walk).

## 2. The two guards

- **Before anything runs**: `tests/batch.rs` reads every `cargo test … -p P --test N …` in
  `tools/batch.sh` (its `for t in …` loops expanded) and in `doc/todo/02`, finds `N`'s file under
  `crates/`, `raster/crates/` or `tools/`, and counts its `#[test]` functions ignored and not by
  their own attribute lines — a doc comment mentioning `#[ignore]` and a `cfg_attr(miri, ignore)`
  are not an ignore. A line whose flag selects none of them fails, naming the line. Against the
  tree before the fix it named `doc/todo/02`'s `--test gate` and nothing else.
- **While it runs**: `gates()`'s `run` fails a `cargo test` line whose output has no
  `test result` with a non-zero `passed`, logged as `ran zero tests`.

## 3. Cost

The attribute reader is a line reader, not a parser: a test whose attributes are separated from
the previous item's closing brace by something other than attributes and comments may be
misread. Every gate file in the tree reads as its authors meant (the calibration test holds the two
spellings that are not an ignore). A command written with `-p` after `--test`, or in a document
other than the two read, is not checked.
