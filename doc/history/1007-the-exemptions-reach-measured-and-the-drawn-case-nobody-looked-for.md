# 1007 — The exemption's reach, measured, and the drawn case nobody looked for

Branch `batch-1006-1011`, worktree `/home/AI/pdf-viewer-rounds`. Topic: the reach of ISO 19005
section 6.2.2's unreferenced-named-resource exemption — what session 1001 made readable, and the
two rewrites `doc/questions/Q62` says it left unreachable. ADR 1026.

## What was done

1. **`crates/pdf-archive/examples/withdrawn.rs`** — a new instrument. Per requirement identifier,
   in how many documents the predicate found a place, in how many the exemption withdrew some or
   all of them, and in how many a place stood. Bounded and every bound reported by name
   (`--max-mb`, `--threads`, `--skip`, panics caught and counted, documents over ten seconds
   named). It prints beside each narrowed row what the new audit says that row's subclause is, so a
   contradiction raises itself.
2. **`crates/pdf-archive/src/withdrawal.rs`** — the per-subclause reading, 147 entries over both
   parts: `Kept`, `Resource`, `NotAResource`, `NoFileCanFail`, each with its reason. Six tests
   hold it to `coverage` in both directions, to `reach::exemption_narrows` for the mechanical
   half, to the requirement table for `NoFileCanFail`, and to Q62's two subclauses by name.
3. **`reach::components`' comment corrected**, and `tests/reach.rs` grew a test for it: an annex
   clause is narrowed like every other requirement of the document, which is what the code did and
   the opposite of what its comment claimed.
4. **The measurement** — the seven read corpora in full, and the crawl as far as a bound and the
   batch's clock allowed — and the conversion probe that prices Q62's two rewrites. ADR 1026 §5
   has the figures and what is left to walk; nothing here repeats them.

## What it found

- **No over-narrowing.** Every row the exemption was seen to narrow sits in a subclause the reading
  calls `Resource`.
- **Both of Q62's rewrites have documents**, and the drawn case is where they are: the converter
  applies `PostScriptXObject` on two corpus documents and `SymbolicTrueTypeEncodingRemoved` on
  seven, with a file written in each case. The question stays open — it is the owner's — and ADR
  1026 §5.3 prices its three options with names and counts.
- **One corpus document costs more than the rest of its corpus together**: the Isartor
  implementation-limits file, over eighteen minutes against six targets. Skipped by name, reported,
  and named for whoever takes the cost item.
- **A long walk that prints at the end loses everything to a bound.** The first pass over the crawl
  reached 87 000 of 89 286 documents and then hit `bounded.sh`'s twelve-gibibyte `RLIMIT_DATA` with
  eight workers, and aborted with no report. The instrument prints a report per *root*, so the
  second pass names the crawl's subdirectories and keeps what it has walked; the lesson is written
  at `sweep`.

## Gates

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets` under `RUSTFLAGS="-D warnings"`
and `cargo test --workspace --doc` each fail on **another round's in-flight files** in this shared
worktree (`crates/pdf-transform/`, `raster/crates/raster-scene/`, `crates/pdf-model/src/icc.rs`);
`pdf-archive` is clean under all three, and `cargo check --workspace` passes. `cargo nextest run
--workspace` is green, both `fuzz/` lines are green, `cargo test -p conformance` is green, and
`tools/state.sh archive` prints `over` 0 on all six targets with the converter conforming in and
conforming out — the two columns this round could have moved and did not.

## Files touched

`crates/pdf-archive/src/withdrawal.rs` (new), `crates/pdf-archive/examples/withdrawn.rs` (new),
`crates/pdf-archive/src/reach.rs`, `crates/pdf-archive/src/lib.rs`,
`crates/pdf-archive/tests/reach.rs`, `doc/todo/62-the-exemption-no-row-states.md`,
`doc/adr/1026-what-the-exemption-reaches-measured-and-two-rewrites-that-had-documents.md`, this
file.

Nothing outside `crates/pdf-archive/` and `doc/` was edited; `crates/pdf-transform/` was run and
never touched.
