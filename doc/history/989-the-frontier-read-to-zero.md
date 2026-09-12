# 989 — The frontier read to zero, and the edition sweep that became a test

Date: 2026-09-12. Branch `round-945/the-fifth-round`, from `0dde6224`. Stream: the PDF/A
validator, `crates/pdf-archive`. Five sibling rounds were live in the same working tree
throughout (984 read-only, 986 in `crates/pdf-transform`, 987 in `pdf-model`'s colour files, 988
in the renderers and transparency, 990 in `pdf-syntax`, the viewer crates and `tools/`), none of
them in `crates/pdf-archive/`.

## What the round was asked to do, and what it did

Three things: read the sentence-level frontier to zero, depth over breadth; give `coverage.rs`
the `Carried::Clarified` variant two rounds had said the second row of its kind should buy; and
finish the base-standard sweep as a test.

- **The frontier is empty.** Forty-one subclauses read sentence by sentence — ISO 19005-2 6.6.1
  to 6.11 and ISO 19005-4 6.7.1 to 6.15 — with a verdict per normative sentence, the eleven
  recorded at subclause level as stating no requirement included. **One normative sentence had no
  row**: ISO 19005-2 section 6.6.2.3.3's rule that a property's declared value type names one the
  XMP Specification defines or one the same extension schema defines, sitting between two tables
  of a subclause three rows already cite. ADR 1010 section 1 has it and the two verdicts that
  could have gone the other way.
- **`Carried::Clarified { by, ground }` exists**, with `Ground::Resolution`, `Ground::Note` and
  `Ground::BaseStandard`, because the round found a third row of the shape with a third ground:
  `embedded-files/associated-file-media-type` rests on ISO 32000-2 §14.13.2 through section 5.1
  alone. The two rows that used to disclaim themselves inside a `By` are moved onto it, and a
  test holds a `Resolution` against `crate::clarification` and refuses a `By` whose sentence
  begins "not ". ADR 1010 section 2.
- **The base-standard sweep is a test.** `crates/pdf-archive/src/editions.rs` holds every cited
  ISO 32000-2 number whose counterpart in ISO 32000-1:2008 is not the same number under the same
  title, and `every_citation_resolves_in_the_edition_a_part_two_file_adheres_to` scans every `§`
  in the crate against both conversions' headings, both ways. Run mechanically it lists nineteen
  numbers where session 975's hand sweep listed eight; ADR 1010 section 3 has the table of the
  eleven it added and why each was missed. `target.rs`'s "§5's three levels" — ISO 19005-2's
  clause 5, reading as ISO 32000-2's — is spelled out.

## Requirement identifiers

**One added, none removed, none re-keyed, none promoted.**

- `metadata/extension-property-value-types-are-defined` — `Check::Unchecked`, ISO 19005-2
  section 6.6.2.3.3 only, with the price in its reason.

No row entered `pdf_transform::archive::unconsidered()`; `crates/pdf-transform/src/archive/
decision.rs` and `tests/archive_unconsidered.txt` were not touched by this round (the former is
round 986's this session and shows as modified by it).

## The figures, off the run

The structural audit (`cargo run -q -p pdf-archive --example frontier`): 220 subclauses listed,
**176 of them read sentence by sentence** against 135 before, **571 normative sentences** against
435 — 402 carried by a row, 3 carried on a clarification, a NOTE or the base standard, 32
restated at another clause, 39 scoping, 95 stating no requirement. **No subclause has no
sentence-level reading**, against 41 before, and the example prints `none`.

The coverage census (`cargo run -q -p pdf-archive --example targets`):

| target | binds | checked | unchecked | processor | clarified |
|---|---|---|---|---|---|
| PDF/A-2b | 181 | 126 | 18 | 36 | 1 |
| PDF/A-2u | 183 | 128 | 18 | 36 | 1 |
| PDF/A-2a | 192 | 135 | 20 | 36 | 1 |
| PDF/A-4 | 167 | 118 | 18 | 31 | 0 |
| PDF/A-4f | 167 | 117 | 18 | 32 | 0 |
| PDF/A-4e | 170 | 116 | 18 | 36 | 0 |

One `unchecked` row more on each part 2 target; part 4 is unchanged.

The corpus harness (`tools/bounded.sh --data 12 --tree 12 -- cargo test --profile gates -p
pdf-archive --test corpus -- --ignored --nocapture`), exit 0, 17 s, peak 0.70 GiB over the
process tree:

| target | agreed | missed | over | settled | elsewhere | unreadable |
|---|---|---|---|---|---|---|
| PDF/A-4 | 473 | 0 | 0 | 8 | 6 | 0 |
| PDF/A-4f | 9 | 0 | 0 | 0 | 2 | 0 |
| PDF/A-4e | 17 | 0 | 0 | 1 | 1 | 0 |
| PDF/A-2b | 971 | 1 | 0 | 7 | 7 | 0 |
| PDF/A-2u | 21 | 0 | 0 | 1 | 0 | 0 |
| PDF/A-2a | 27 | 0 | 0 | 0 | 0 | 0 |

`over` is 0 on all six, before and after. The one miss is the standing `6-6-2-3-3-t03-fail-b`,
where errata A029 makes our answer the right one. The one row added is `Check::Unchecked`, so no
verdict could move, and none did.

## Gates

Green: `cargo fmt --all --check`, `RUSTFLAGS="-D warnings" cargo clippy -p pdf-archive
--all-targets` (silent), `cargo nextest run -p pdf-archive` (195 passed, the seven new tests
among them), `cargo test --workspace --doc`, both `fuzz/` lines, `cargo test -p conformance`
(234 passed, plus `tests/conformance.rs`'s 6), and the corpus harness at exit 0 under
`tools/bounded.sh`.

**The conformance gate was red once, on this round's own file, and the shape is worth
recording**: `every_citation_names_a_clause_that_exists` reported two `'§'` character literals
in `editions.rs`'s scanner as citations it could not read. The scanner that reads every `§` in
the tree read the scanner that reads every `§` in this crate; the sign is spelled by its code
point now (`const SIGN: char = '\u{a7}'`), and ADR 1010 section 4 has the general form.

**Two whole-workspace lines could not be run green, for reasons outside this stream**, both in
files round 990 was holding open at the time — first `crates/pdf-syntax/tests/zz_find_990.rs`,
then, on a rerun after that file had moved on, `tools/conformance/tests/state_sections.rs`:

- `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` stops on an unfulfilled lint
  expectation in that test file.
- `cargo nextest run --workspace --no-fail-fast` is **4390 run, 4389 passed, 1 failed, 35
  skipped**; the one is `conformance::state_sections
  every_gate_line_the_sequence_states_is_one_the_state_script_runs`, which reads `tools/state.sh`,
  a file the same round shows as modified. On the first workspace run a `pdf-transform`
  signature test failed as well and passed when re-run alone and on the second run — round 986's
  crate, mid-edit at the time.

Neither is reachable from anything this round touched. Whoever merges owns `doc/todo/02`
section 2's sequence on `main`.

## Files touched

- `crates/pdf-archive/src/coverage.rs` — forty-one readings, the `Clarified` variant and
  `Ground`, the module comment, the tests
- `crates/pdf-archive/src/editions.rs` — new
- `crates/pdf-archive/src/lib.rs`
- `crates/pdf-archive/src/table/metadata.rs` — one row, one paragraph of the header
- `crates/pdf-archive/src/target.rs` — one doc comment
- `crates/pdf-archive/src/table/fonts.rs`, `table/graphics.rs`, `src/survey.rs` — one sentence
  apiece pointing at the test
- `crates/pdf-archive/examples/frontier.rs` — a column, and a line for an empty frontier
- `doc/adr/1010-the-frontier-read-to-zero-and-the-sweep-that-became-a-test.md` — new
- this file

`crates/pdf-model/src/der.rs` and `signature.rs` were read and not edited.

## What the next round should know

- **There is no frontier to move.** What is left in this crate's audit is not a region but the
  rows themselves: `--example targets` counts the `Unchecked` ones per target, and each carries
  its price. The new 6.6.2.3.3 row is the cheapest of them — two vocabularies of names, both in
  documents this tree holds — and closing it is a predicate plus the converter's census row.
- **`Carried::Clarified` has three grounds and three rows**, one per ground. A fourth row of the
  shape goes onto whichever ground it rests on; a fourth *ground* is an argument, not a variant.
- **The edition table is held by a test, so a shifted citation fails the build by name** with
  both editions' titles printed. A round adding a `§` to this crate that fails it adds a row to
  `editions::SHIFTS` after reading both headings, never after reading one.
- **The single-signer row's blocker is still in `pdf-model`**: a DER reader that refuses, or
  reports, X.690 clause 10's departures. Nothing here changed that price.
