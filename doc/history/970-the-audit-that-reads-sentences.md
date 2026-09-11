# 970 — The audit that reads sentences

Date: 2026-09-11. Branch `round-945/the-fifth-round`, from `8eac6aa2`. Stream: the PDF/A
validator, `crates/pdf-archive`. Four sibling rounds were live in the same working tree
throughout (the viewer, the converter in `crates/pdf-transform`, the instruction documents, and
general improvement).

## What the round was asked to do, and what it did

Two things: close what could be closed among session 965's twenty-one new rows, and take
`coverage.rs` from subclause-level to sentence-level with a stated frontier.

- **The sentence-level layer is committed as data and checked by five more tests.**
  `Sentence`, `Carried` and `Reading` sit beside `Subclause` and `Binding`; `frontier()` computes
  what has not been read, so the frontier is printed by a command rather than claimed by a
  paragraph. `cargo run -p pdf-archive --example frontier` is that command, and it is new.
- **Session 965's stated limit turned out to be one step short.** Its module comment said
  sentence-level coverage "cannot be committed to this tree at all" because it would have to
  enumerate the standard's sentences. The licence forbids reproducing the standard's *words*; a
  sentence-level audit needs a count and a mapping, and this crate's own paraphrase beside each —
  which is what `Requirement::asks` has always been. ADR 0981 section 1.
- **The pass found eight normative sentences with no row, in four subclauses.** Two of those
  subclauses — ISO 19005-4 Annex A.1 and Annex B.1 — were recorded as stating only scope, and each
  also requires a conforming processor of that flavour to read every plain PDF/A-4 file. Five rows
  added, all `Check::Processor`.
- **`conformance/no-deprecated-features` was re-priced rather than closed**, and the reason now
  says why: the route it named needs a document-to-Arlington-object resolver that no crate in this
  tree has, and a mechanical per-key reading contradicts ISO 19005-4 section 6.1.11 on `UR3`. The
  figures are in ADR 0981 section 4.
- **Nothing else among the twenty-one could be closed without a second crate's commit.** The two
  signature rows ADR 0972 called "one predicate away" still are, and a predicate is a row of
  `crates/pdf-transform/tests/archive_unconsidered.txt`, which this round did not own.

## Requirement identifiers

**Five added, none removed, none re-keyed.** All five are `Check::Processor`, which
`pdf_transform::archive::unconsidered()` does not admit, so
`crates/pdf-transform/tests/archive_unconsidered.txt` is unaffected and unedited —
`cargo nextest run -p pdf-transform` is 358 passed, including
`the_unconsidered_requirements_are_the_ones_this_file_names`.

Added: `conformance/undescribed-features-are-ignored`,
`conformance/processor-reads-every-conforming-file`,
`conformance/a-part-two-reader-also-reads-part-one`,
`conformance/an-embedded-files-processor-reads-the-plain-profile`,
`conformance/an-engineering-processor-reads-the-plain-profile`.

One existing row's `asks` was narrowed: `conformance/processor-behaviour` no longer claims the
"ignore undescribed features" sentence for both parts, because part 4 states it as a
recommendation. The identifier is unchanged.

## The figures, off the run

The corpus harness (`tools/bounded.sh -- cargo test --profile gates -p pdf-archive --test corpus
-- --ignored`):

| target | agreed | missed | over | settled | elsewhere | unreadable |
|---|---|---|---|---|---|---|
| PDF/A-4 | 473 | 0 | 0 | 8 | 6 | 0 |
| PDF/A-4f | 9 | 0 | 0 | 0 | 2 | 0 |
| PDF/A-4e | 17 | 0 | 0 | 1 | 1 | 0 |
| PDF/A-2b | 971 | 1 | 0 | 7 | 7 | 0 |
| PDF/A-2u | 21 | 0 | 0 | 1 | 0 | 0 |
| PDF/A-2a | 27 | 0 | 0 | 0 | 0 | 0 |

`over` is 0 on all six, before and after. The one miss is `6-6-2-3-3-t03-fail-b`, where errata
A029 makes our answer the right one.

The coverage census (`cargo run --profile gates -p pdf-archive --example targets`):

| target | binds | checked | unchecked | processor | clarified |
|---|---|---|---|---|---|
| PDF/A-2b | 177 | 125 | 17 | 34 | 1 |
| PDF/A-2u | 179 | 127 | 17 | 34 | 1 |
| PDF/A-2a | 188 | 134 | 19 | 34 | 1 |
| PDF/A-4 | 165 | 118 | 18 | 29 | 0 |
| PDF/A-4f | 165 | 117 | 18 | 30 | 0 |
| PDF/A-4e | 168 | 116 | 18 | 34 | 0 |

Nothing moved out of `Implemented`; the whole growth is the five new `Processor` rows.

The structural audit (`cargo run -p pdf-archive --example frontier`), which is new: 220
subclauses listed, 52 of them read sentence by sentence, 128 normative sentences — 100 carried by
a row, 19 restated at another clause, 5 scoping, 4 stating no requirement. **124 subclauses have
no sentence-level reading**, and the example lists them by name.

## Gates

Green: `cargo fmt -p pdf-archive --check`, `RUSTFLAGS="-D warnings" cargo clippy -p pdf-archive
--all-targets`, both `fuzz/` lines, `cargo test -p conformance`, `cargo nextest run --workspace
--exclude spec-errata` (4299 passed), `cargo test --workspace --exclude spec-errata --doc`, and
the corpus harness at exit 0.

**Two whole-workspace lines could not be run green, for reasons outside this stream**, both in
files a sibling round is holding open:

- `cargo fmt --all --check` is red in `tools/conformance/src/ledger.rs` at two places.
- `cargo nextest run --workspace` does not compile: `tools/spec-errata/src/lib.rs` names
  `conformance::ledger::TECHNICAL_CLAUSES`, which that crate no longer defines.

Neither is reachable from anything this round touched. Whoever merges owns `doc/todo/02`
section 2's sequence on `main`.

## Files touched

- `crates/pdf-archive/src/coverage.rs`
- `crates/pdf-archive/src/lib.rs`
- `crates/pdf-archive/src/table/file_structure.rs`
- `crates/pdf-archive/examples/frontier.rs` — new
- `doc/adr/0981-the-audit-that-reads-sentences-and-the-two-annexes-that-hid-a-processor.md` — new
- this file

## What the next round should know

- **The frontier is a list, not a number**, and `--example frontier` prints it. Clause 6.2 of both
  parts is the next block and the largest: the JPEG 2000 subclauses, the output intent, the
  extended graphics state and the uncalibrated colour spaces each state more sentences than they
  have rows, which after this round's experience is a ranking rather than a finding.
- **The two signature rows are still one predicate away, and the predicate is a two-crate
  commit.** `signatures/digest-covers-the-whole-file` and
  `signatures/signature-is-a-single-signer-cms-object` each need a `Check::Implemented`, and every
  `Implemented` row enters `pdf_transform::archive::unconsidered()` unless the converter's
  decision tables answer it. A round that owns both crates can close them in an afternoon; a round
  that owns one cannot close them at all.
- **`crates/pdf-syntax/Cargo.toml` declares `pdf-spec` and no file in that crate uses it.** Found
  while pricing `conformance/no-deprecated-features`; not this stream's to fix, and it is part of
  why the Arlington route looked closer than it is.
