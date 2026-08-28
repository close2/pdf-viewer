# 810 — A fuzz run that exits zero without fuzzing

Date: 2026-08-28. Branch `round-810`, from `main` at `120951e7`. Parallel round, worktree `r810`.
ADR: [0742](../adr/0742-a-fuzz-run-that-exits-zero-without-fuzzing.md).
Touched: `fuzz/Cargo.toml`, nine of the fifteen `fuzz/fuzz_targets/*.rs`,
`tools/conformance/tests/workspaces.rs`, `tools/worktree.sh`, `tools/state.sh`,
`doc/todo/02-every-round.md` (§2), `doc/verify.md`,
`doc/traps/instruments-and-reports.md` (trap 24, new; trap 23 extended), `doc/HANDOVER.md` (its
index), and two new files — `tools/fuzz.sh` and `doc/adr/0742` — plus this one.

## Which of ADR 0739's two holes were taken

**Both**, because they turned out to be one shape seen twice and neither is large on its own once
the measurement is in hand. ADR 0739 left them in writing: `fuzz/` was unlinted, and a fuzz target's
exit status says nothing about whether it fuzzed.

## Hole one: `clippy` had never judged a fuzz target

`fuzz/Cargo.toml` took no `[lints] workspace = true`. Adding it surfaced **33 findings across nine
of the fifteen targets**, in three kinds with three different answers:

- **Five are arithmetic in a target's own counters**, and they are the ones that mattered.
  `fuzz/Cargo.toml` sets `overflow-checks = true` deliberately, so a target's own `+=` is an abort
  waiting to be reported as a crash in the parser under test. Each is now saturating, with the
  sentence saying why saturation cannot hide what the assert beside it catches.
- **Twenty are `panic!` and `expect`**, which are a fuzz target's vocabulary rather than a defect —
  crate-level `#![expect(…, reason = …)]` per target, the form `crates/pdf-model/tests/actions.rs`
  already uses. `clippy.toml`'s `allow-panic-in-tests` does not reach a `#![no_main]` binary, which
  is correct: an exemption at the file makes the exception visible.
- **The rest is pedantic style** — backticked `` `XObject` ``, three `match_same_arms` whose
  comments are what merging would delete, one `items_after_statements`.

§2's `cargo check --manifest-path fuzz/Cargo.toml --bins` became
`RUSTFLAGS="-D warnings" cargo clippy --manifest-path fuzz/Cargo.toml --all-targets`; the core stays
six lines because `clippy` answers the compiling question too.

**And the workspaces test does now demand a lint line, with an argument.** 807 wrote it not to, on
the ground that a `check` line answers the compiling half. The reason to extend it is 807's own
applied once more: that gate's population is *workspaces* because a workspace is the mechanism that
produced the defect, and the mechanism has now produced it twice — for formatting and for linting —
by exactly the same rule. So the gate gained two arms: every root must be named by a `cargo clippy`
line **under `RUSTFLAGS="-D warnings"`** (ADR 0450's flag is part of the requirement), and every
root's `[workspace.lints.*]` tables must state what the tree's own root states, because cargo cannot
inherit lints across workspaces and the copy is what drifts. 807's refusal to generalise beyond
workspaces still stands.

## Hole two: what an unseeded target is actually worth

Every target run twice — 30 000 executions from an empty directory, and its own corpus executed once
(`-runs=0`), which is the seeds' coverage rather than a session's. The table is in ADR 0742. Three
findings:

**`page` from nothing reaches cov 103 and ft 182 — the same two figures as `document`, to the
unit.** The two share a prefix and diverge only once a document parses well enough to have a page
tree, so `page` never enters `pdf_model::interpret`, which is the whole reason ADR 0264 built it.
Its corpus takes it to ft 218 862.

**A worktree had no fuzz corpus at all.** `fuzz/corpus` and `fuzz/artifacts` are gitignored and
`tools/worktree.sh` did not link them, so every fuzz run a parallel round has ever made started from
nothing — measured, not reasoned: the sibling round's worktree has no such directory and this
round's first pass reported `files=0` for all fifteen targets. Both are now linked, and a fresh
`tools/worktree.sh open` was run to check it.

**`display_list` had never been seeded here at all**, because its seeder is an example binary rather
than a Python script. Seeded from the pdf.js documents it goes from cov 631 / ft 906 to cov 1796 /
ft 7294.

`tools/fuzz.sh` is the instrument. It runs **`doc/verify.md`'s own line** for the target — that file
owns the invocation, so the two cannot drift, and a target with no line is refused — and asks the two
questions the bare command does not: an empty corpus stops the run before it starts, and a final
`ft` of zero fails it afterwards. `tools/fuzz.sh --list` prints every target with the seeds it has
*here*, which is where the two findings above were visible in one second. `tools/state.sh counts`
gained the same fact as a number.

## Trap 13, nine plants, above commit `4b4bb2a6`

Each planted, run, reverted; every message distinct, so no arm swallows another's case (796's
lesson). The first two are 807's arms re-checked after the `Invocation` refactor.

| plant | gate says |
|---|---|
| §2's `cargo fmt --manifest-path fuzz/…` deleted | `no cargo fmt line … : ["fuzz/Cargo.toml"]` |
| §2's `clippy --manifest-path fuzz/…` deleted | the **compiling** arm, naming it |
| that line changed back to `cargo check … --bins` | the **linting** arm — compiled, not linted |
| the same line with its `RUSTFLAGS` prefix removed | the linting arm again |
| `RUSTFLAGS` removed from *every* clippy line | `yielded 2 formatting, 4 compiling and 0 linting lines … measuring nothing` |
| `panic = "warn"` deleted from `fuzz/Cargo.toml` | `divergent … ["fuzz/Cargo.toml: clippy.panic=\"warn\""]` |
| `shadow_unrelated = "warn"` added to it | the same arm, the other direction |
| `tokens += 1` restored in `fuzz_targets/lexer.rs` | `clippy --workspace` exits **0**; the new fuzz line exits 101 and names it |
| `page` under `-fork=6` against an empty corpus | `#59025: cov: 0 ft: 0 corp: 0 … exiting: 0` — round 800's sentence, reproduced |

The last is also the calibration of `tools/fuzz.sh`'s own report, and it corrected it. An **ordinary**
run's `ft` is never zero — libFuzzer executes the empty input at `INITED`, and `display_list` from
nothing reports `cov: 31 ft: 32`, which the wrapper passes. The zero is a **fork-mode parent's**,
reporting a shared corpus that started empty; run through the wrapper, `cargo-fuzz` exits 0 and
`tools/fuzz.sh` exits 1. Trap 11 applied to this round's own instrument, and it changed what the
check is documented to mean.

## Gates — a fifth round, so §2 whole and §5

| line | result |
|---|---|
| `cargo fmt --all --check` | silent |
| `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` | silent, beside the documented `proc-macro-error2` future-compat note |
| `cargo nextest run --workspace` | 2782 run, 2782 passed, 18 skipped |
| `cargo test --workspace --doc` | passed |
| `cargo fmt --manifest-path fuzz/Cargo.toml --check` | silent |
| `RUSTFLAGS="-D warnings" cargo clippy --manifest-path fuzz/Cargo.toml --all-targets` | silent — **the first round in which that sentence means anything** |
| corpus | passed |
| oracle | passed, 3 tests, 110 s |
| text_extraction | passed, 4 tests |
| selection_census | passed |
| accessibility_census | passed, 0 panicked |
| dates, xmp, jpeg2000 | passed |
| quorra corpus | passed |
| fixed_documents | passed, 41 checked, 0 absent |
| `cargo test -p conformance` | passed |
| §5 | eight binaries rebuilt and installed into `target/` |

**The machine was not quiet**, and this is said rather than hidden: two sibling rounds and a quorra
run were building throughout, at a load average between 12 and 34 on 24 cores. §2's own warning is
that a gate spawning a reference renderer is a measurement of two programs — the oracle passed at
110 s against the 57 s §2 quotes for an unloaded run, and its verdicts are page lists rather than
timings, so the pass stands and the duration does not.

## The §4 sweeps, before and after, every delta accounted

Twenty-one sweeps over a pristine `git worktree` of `120951e7` with its own build directory (the
second method in `doc/todo/01`, which touches no file here), and again over this branch. Seven files
differ and none is a finding:

- **`counts`** reads 5 more sentences governing a ledger word — this round's prose. No verb added.
- **`inapplicable`** counts one more file naming `C`; the hit rows are identical.
- **`ledger`** differs only in the absolute path it prints. 875 rows, 0 new, both sides, and
  `--bin ledger` regenerates nothing.
- **`overtaken`** reads 630 decision records against 629 — this round's ADR.
- **`parts`** quotes the handover's trap table, which gained trap 24, and reports its one
  `doc/todo/02` hit twelve lines lower because this round added lines above it. Same hit.
- **`pointers`**: `98 absent` on both sides, which is the finding-shaped number. The live and
  not-carried columns move because the *base checkout* carries less gitignored data than a worktree
  does — `doc/corpora`'s four submodules and `fuzz/corpus` are linked into one and not the other.
- **`applied`** reads 76 more places, `0` naming an erratum on both sides, and the `#NNN` token
  count is identical at 1521.

`quoted`, `retired` and `unpriced` exit 1 on both sides, unchanged; the first and third take an
argument this run did not give them.

## What is left, precisely

- **`fuzz/corpus` is not recoverable from the history**, because it was never in it. A clone re-seeds
  from the scripts in `fuzz/` and the recipes in `doc/verify.md`, and nothing checks that it did —
  `tools/state.sh counts` reports how many targets are unseeded *here*, which is as close as a
  gitignored directory allows.
- **`seed_confined_wire.py` still stops at discriminant 25**, four questions short, which the
  four-hundred-and-forty-fifth session found and this round did not fix. It is four lines and a
  re-seed, and it is now visible from `tools/fuzz.sh --list` only as a seed count that looks fine.
- **No target was fuzzed for its documented run length in this round.** The measurement was 30 000
  executions and a corpus execution apiece; `doc/verify.md`'s own lengths are an hour for `page` and
  a million runs for `x509` and `confined_wire`. Nothing new was found because nothing new was
  looked for.
