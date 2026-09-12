# 982 — "The entire file" is the file at the moment of signing

Date: 2026-09-11. Branch `round-945/the-fifth-round`, from `9fac224b`. Stream: the PDF/A
validator, `crates/pdf-archive`. Four sibling rounds were live in the same working tree
throughout (979 transparency, 980 colour, 981 fonts, 983 `pdf-syntax` and the conformance
tools), none of them in `crates/pdf-archive/` and none in `crates/pdf-transform/`.

## What the round was asked to do, and what it did

Three things: settle the reading of ISO 19005-2 Annex B.1 that session 975 recorded and
declined to guess at, advance the sentence-level frontier, and spell out the ISO 32000-1:2008
citations in `table/file_structure.rs` that a section sign had been passing off as ISO 32000-2's.

- **The reading is settled, and it is neither of the two session 975 named.** The annex's
  "entire file" is the file as it exists when the digest is computed, and every signature is
  judged against the file *it* was made over — not the newest alone, and not a rule against
  updates after a signature. The sentences it rests on are all in documents this tree already
  held: ISO 19005-2 section 6.4.3's permission (plural signatures, as ISO 32000-1:2008, 12.8.1
  permits them), 12.8.1's own NOTE 1 on incremental updates after signing, Table 252's
  `Changes` entry, and 12.8.2.2.1's permitted changes after a certification signature. ADR 1003
  section 2 is the argument; reading (a) would make section 6.4.3 unreachable.
- **`signatures/digest-covers-the-whole-file` is promoted to `Check::Implemented`**, with a
  predicate of four checks per signature and eight byte-level tests. veraPDF's rule for the
  clause was run on constructed variants as evidence and agrees on every decidable case.
- **The converter has its considered answer in the same commit**: a `REFUSED_BY_NAME` row in
  `crates/pdf-transform/src/archive/decision.rs`,
  `Because::NotBuiltYet(SIGNATURE_RANGE_IS_THE_SIGNERS)`, waiting on the same section 3.6 report
  the two 6.1.12 rows beside it wait on. `tests/archive_unconsidered.txt` is unchanged.
- **The frontier moved by twenty-four subclauses**: ISO 19005-2 clauses 6.3 to 6.5 and
  ISO 19005-4 clauses 6.3 to 6.6, read sentence by sentence. The tranche was already carried
  sentence for sentence; ADR 1003 section 6 records the three things worth keeping.
- **Four citations spelled out**, one of which had been quoting a *shall* of ISO 32000-1:2008,
  F.3.11 under a section sign that names ISO 32000-2, whose F.3.11 says *should*.

## Requirement identifiers

**None added, none removed, none re-keyed. One promoted**:
`signatures/digest-covers-the-whole-file`, `Check::Unchecked` to `Check::Implemented`,
ISO 19005-2 Annex B.1, part 2 only. One reason rewritten without changing its identifier:
`signatures/signature-is-a-single-signer-cms-object` (the blocker is *DER-encoded*, which
`pdf_model::der` deliberately does not enforce, not the signer count).

`decision.rs`: one row added to `REFUSED_BY_NAME` for the promoted requirement, and one
constant, `SIGNATURE_RANGE_IS_THE_SIGNERS`. `cargo test -p pdf-transform --test archive` passes
including `the_unconsidered_requirements_are_the_ones_this_file_names`; the ratchet file is
unedited and still empty.

## The figures, off the run

The corpus harness (`tools/bounded.sh --data 12 --tree 12 -- cargo test --profile gates -p
pdf-archive --test corpus -- --ignored`), exit 0, 21 s, peak 0.72 GiB over the process tree:

| target | agreed | missed | over | settled | elsewhere | unreadable |
|---|---|---|---|---|---|---|
| PDF/A-4 | 473 | 0 | 0 | 8 | 6 | 0 |
| PDF/A-4f | 9 | 0 | 0 | 0 | 2 | 0 |
| PDF/A-4e | 17 | 0 | 0 | 1 | 1 | 0 |
| PDF/A-2b | 971 | 1 | 0 | 7 | 7 | 0 |
| PDF/A-2u | 21 | 0 | 0 | 1 | 0 | 0 |
| PDF/A-2a | 27 | 0 | 0 | 0 | 0 | 0 |

`over` is 0 on all six, before and after. The one miss is the standing `6-6-2-3-3-t03-fail-b`,
where errata A029 makes our answer the right one. The promoted row finds nothing in the corpus:
its only signed PDF/A-2 witnesses are the 6.1.12 files, whose `t02` signatures cover their
files and whose `t01` signature is reachable from `/Perms` alone (ADR 1003 section 3).

The coverage census (`cargo run -q -p pdf-archive --example targets`):

| target | binds | checked | unchecked | processor | clarified |
|---|---|---|---|---|---|
| PDF/A-2b | 180 | 126 | 17 | 36 | 1 |
| PDF/A-2u | 182 | 128 | 17 | 36 | 1 |
| PDF/A-2a | 191 | 135 | 19 | 36 | 1 |
| PDF/A-4 | 167 | 118 | 18 | 31 | 0 |
| PDF/A-4f | 167 | 117 | 18 | 32 | 0 |
| PDF/A-4e | 170 | 116 | 18 | 36 | 0 |

One row moved from `unchecked` to `checked` on each of the three part 2 targets; part 4 is
unchanged, because the row binds part 2 alone.

The structural audit (`cargo run -q -p pdf-archive --example frontier`): 220 subclauses listed,
**135 of them read sentence by sentence** against 111 before, **435 normative sentences**
against 350 — 329 carried by a row, 28 restated at another clause, 28 scoping, 50 stating no
requirement. **41 subclauses have no sentence-level reading**, against 65 before, and the
example lists them by name: both parts' metadata, logical structure, embedded files, optional
content, presentations and `Requirements` subclauses, and part 4's 6.13 to 6.15.

## Gates

Green: `cargo fmt -p pdf-archive --check` and `-p pdf-transform --check`, both `fuzz/` lines,
`cargo test -p conformance` (234 passed), `cargo test --workspace --doc`, `cargo nextest run -p
pdf-archive` (all of it, including the eight new tests and the ten audit tests),
`cargo test -p pdf-transform --test archive` (71 passed), and the corpus harness at exit 0.

**Three whole-workspace lines could not be run green, for reasons outside this stream**, all in
files a sibling round was holding open at the time:

- `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` stops in
  `crates/pdf-syntax/src/lexer.rs:727` (a `single_match` lint), which is round 983's file. The
  same run without `-D warnings` reports no warning in `crates/pdf-archive` or
  `crates/pdf-transform`.
- `cargo nextest run --workspace --no-fail-fast` is **4370 run, 4368 passed, 2 failed, 35
  skipped**; the two are `pdf-syntax::encryption
  a_document_with_a_password_opens_with_it_and_not_without` and `pdf-model::outlines
  a_document_with_an_outline_produces_its_items`, both re-run alone and both still failing, in
  crates this round did not touch.
- `cargo fmt --all --check` reports one diff, in `crates/pdf-model/examples/salvage_census_983.rs`,
  a sibling's untracked file.

Whoever merges owns `doc/todo/02` section 2's sequence on `main`.

## Files touched

- `crates/pdf-archive/src/table/interaction.rs` — the predicate, its tests, two rows' text
- `crates/pdf-archive/src/coverage.rs` — twenty-four readings, the module comment, the region test
- `crates/pdf-archive/src/table/file_structure.rs` — four citations
- `crates/pdf-transform/src/archive/decision.rs` — one refusal row and its sentence
- `doc/adr/1003-the-entire-file-is-the-file-at-the-moment-of-signing.md` — new
- this file

`crates/pdf-model/src/signature.rs` was not touched and needs no change for this reading.

## What the next round should know

- **The frontier is clause 6.6 onwards in part 2 and 6.7 onwards in part 4**, and
  `--example frontier` lists it by name. Both parts' metadata subclauses are the largest block
  left, and ISO 19005-2 section 6.6.2.3.3 is where one row stands for four tables of required
  fields — the place session 975 named as the one to expect the sentence count to exceed the
  row count.
- **`Carried::Clarified` is now owed by the second row of its kind and was not bought here.**
  `annotations/appearance-dictionary-present-from-base-standard` carries a rule ISO 19005-4
  section 6.3.3 states only in a NOTE; ADR 1003 section 6 says why this one is not the same
  shape as `named-resources-are-defined` and what a variant would have to distinguish.
- **The single-signer row's blocker is in `pdf-model`**: a DER reader that refuses, or reports,
  X.690 clause 10's departures. `Value::had_indefinite_length` is the one such report that
  exists.
- **A round widening the signature population past the form's fields will meet
  `6-1-12-t01-pass-a`** and its part 4 twin — a signature reachable from `/Perms` alone whose
  range runs past the file. It is `signatures/signatures-use-signature-fields`'s finding first.
