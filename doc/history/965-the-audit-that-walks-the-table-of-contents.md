# 965 — The audit that walks the table of contents

Date: 2026-09-11. Branch `round-945/the-fifth-round`, from `249c3a58`. Stream: the PDF/A validator,
`crates/pdf-archive`. Three sibling rounds were live in the same working tree throughout (the
viewer, the converter in `crates/pdf-transform`, and a rewrite of the instruction documents).

## What the round was asked to do, and what it did

Not another sweep of the rows that exist — the two rounds before it had done that twice and found
them accurate. This round asked the question neither sweep could: **is the table complete?** It
walked ISO 19005-2 and ISO 19005-4 from their own tables of contents, subclause by subclause,
including both parts' normative annexes, and asked of each: does a row bind it?

- **The audit is committed as data and checked by tests**, not written down as prose:
  `crates/pdf-archive/src/coverage.rs` records one verdict per subclause of both parts, and five
  tests hold it against `crate::table` in both directions.
- **It found twenty-one normative sentences with no row at all**, including the whole of
  ISO 19005-2's normative Annex B and the transparency method its normative Annex A states — which
  the table already silently depended on. All twenty-one now have rows.
- **The frontier is: there is none** inside clause 5, clause 6 or the normative annexes of either
  part. Clauses 1 to 4 state no requirement and are the one boundary.

ADR 0972 has the findings, the argument for the instrument, and why every new row is
`Check::Processor` or `Check::Unchecked` rather than a predicate.

## Requirement identifiers

**Twenty-one added, none removed, none re-keyed.** The converter's census
(`crates/pdf-transform/tests/archive_unconsidered.txt`) is unaffected and unedited: it lists
requirements whose check is a predicate a document can fail, and every new row here is `Processor`
or `Unchecked`. `cargo test -p pdf-transform --test archive` passes, including
`the_unconsidered_requirements_are_the_ones_this_file_names`.

Added: `conformance/no-deprecated-features`,
`conformance/the-version-number-does-not-decide-conformance`, `conformance/processor-behaviour`,
`file-structure/undescribed-data-never-renders`,
`file-structure/unreferenced-objects-never-influence-rendering`,
`file-structure/linearization-permitted`, `graphics/destination-profile-alternate-ignored`,
`graphics/jpeg2000-best-colour-space-specification-used`,
`graphics/transparency-determined-by-the-parts-own-method`,
`graphics/blend-modes-processed-as-the-base-standard-defines`,
`annotations/appearance-rendered-without-the-other-entries`,
`signatures/signatures-use-signature-fields`, `signatures/signing-does-not-break-conformance`,
`signatures/timestamped-file-follows-the-base-standard`, `signatures/digest-covers-the-whole-file`,
`signatures/signature-is-a-single-signer-cms-object`,
`signatures/revocation-information-is-a-signed-attribute`,
`signatures/signature-handlers-available`,
`signatures/signatures-validated-as-the-annex-describes`,
`actions/a-processor-that-declines-scripts-says-so`,
`actions/on-instantiate-script-only-on-explicit-user-action`.

## The figures, off the run

The corpus harness (`--profile gates -p pdf-archive --test corpus -- --ignored`), after the change:

| target | agreed | missed | over | settled | elsewhere | unreadable |
|---|---|---|---|---|---|---|
| PDF/A-4 | 473 | 0 | 0 | 8 | 6 | 0 |
| PDF/A-4f | 9 | 0 | 0 | 0 | 2 | 0 |
| PDF/A-4e | 17 | 0 | 0 | 1 | 1 | 0 |
| PDF/A-2b | 971 | 1 | 0 | 7 | 7 | 0 |
| PDF/A-2u | 21 | 0 | 0 | 1 | 0 | 0 |
| PDF/A-2a | 27 | 0 | 0 | 0 | 0 | 0 |

`over` is 0 on all six, before and after. The one miss is `6-6-2-3-3-t03-fail-b`, where errata A029
makes our answer the right one.

The coverage census (`cargo run --profile gates -p pdf-archive --example targets`):

| target | binds | checked | unchecked | processor | clarified |
|---|---|---|---|---|---|
| PDF/A-2b | 174 | 125 | 17 | 31 | 1 |
| PDF/A-2u | 176 | 127 | 17 | 31 | 1 |
| PDF/A-2a | 185 | 134 | 19 | 31 | 1 |
| PDF/A-4 | 164 | 118 | 18 | 28 | 0 |
| PDF/A-4f | 163 | 117 | 18 | 28 | 0 |
| PDF/A-4e | 166 | 116 | 18 | 32 | 0 |

The table is 239 rows: 174 `Implemented`, 38 `Processor`, 26 `Unchecked`, 1 `OutsideValidation`.
Nothing moved out of `Implemented`; the whole growth is the twenty-one rows.

## Gates

`cargo fmt --all --check`, the two `fuzz/` lines, `cargo test -p conformance`, the doc tests and the
corpus harness are all green. `RUSTFLAGS="-D warnings" cargo clippy -p pdf-archive --all-targets` is
silent and `cargo nextest run -p pdf-archive -p pdf-model -p pdf-syntax -p pdf-font -p pdf-spec -p
conformance` is 2184 passed.

**The whole-workspace lines could not be run green, for reasons outside this stream**: the
converter sibling's `crates/pdf-transform/src/archive/rewrite.rs` does not compile at the moment of
this writing (`no field colorants on type Owed`), and the viewer sibling's
`crates/pdf-font/src/tounicode.rs` carries one `clippy::cast_possible_truncation`. Neither is
reachable from anything this round touched, and both are mid-round work in the same tree. Whoever
merges owns `doc/todo/02` section 2's sequence on `main`.

## Files touched

- `crates/pdf-archive/src/coverage.rs` — new
- `crates/pdf-archive/src/lib.rs`
- `crates/pdf-archive/src/survey.rs`
- `crates/pdf-archive/src/table/file_structure.rs`
- `crates/pdf-archive/src/table/graphics.rs`
- `crates/pdf-archive/src/table/interaction.rs`
- `doc/adr/0972-the-audit-that-walks-the-table-of-contents.md` — new
- this file
