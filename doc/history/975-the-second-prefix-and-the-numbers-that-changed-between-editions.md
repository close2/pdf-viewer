# 975 — The second prefix, and the numbers that changed between editions

Date: 2026-09-11. Branch `round-945/the-fifth-round`, from `bb77086d`. Stream: the PDF/A
validator, `crates/pdf-archive`. Four sibling rounds were live in the same working tree
throughout (the viewer, the converter in `crates/pdf-transform`, the instruction documents, and
general improvement).

## What the round was asked to do, and what it did

Three things: advance the sentence-level frontier `--example frontier` prints, close the two
signature rows two earlier rounds called "one predicate away", and continue the base-standard
sweep for part-2-binding rows justified by ISO 32000-2 sentences ISO 32000-1:2008 does not state.

- **Clause 6.2 of both parts is read sentence by sentence** — fifty-nine subclauses: graphics,
  the output intent, all five colour space subclauses, the extended graphics state, flatness,
  images and JPEG 2000, XObjects, transparency and every font subclause. The region read is still
  a *prefix of each part in that part's own order*, plus the annexes, which is the property that
  makes it reviewable against a copy.
- **Two normative sentences both parts state had no row**, and both are processor obligations
  found by counting a `Bound` subclause's sentences rather than asking whether it had rows:
  the PDF/A output intent is the default blending colour space (ISO 19005-2 section 6.2.10,
  ISO 19005-4 section 6.2.9), and a conforming processor renders with the embedded font programs
  rather than a substituted face (section 6.2.11.4.1, section 6.2.10.4.1). The second is an
  obligation on the program this repository *is*.
- **The two signature rows were not closed, and the reason is a finding rather than a refusal.**
  Neither is one predicate away. ADR 0986 section 3 has both arguments; the short form is that
  `signatures/digest-covers-the-whole-file` is blocked by a *reading* nobody had priced — whether
  ISO 19005-2 Annex B.1's "entire file" forbids a conforming file from carrying an incremental
  update after a signature — and that `signatures/signature-is-a-single-signer-cms-object` was two
  requirements in one, whose RFC 2315 half is now a row of its own so that the countable half is
  no longer hostage to it.
- **The base-standard sweep found eight clause numbers** that resolve, in the edition PDF/A-2
  adheres to, to a different subclause — and three citations that were simply wrong in either
  edition. ADR 0986 section 4 has the table and where each correction went.

## Requirement identifiers

**Three added, none removed, none re-keyed, none promoted to `Check::Implemented`.**

- `graphics/output-intent-is-the-default-blending-space` — `Check::Processor`, both parts
  (ISO 19005-2 section 6.2.10, ISO 19005-4 section 6.2.9).
- `fonts/embedded-programs-are-what-a-processor-renders` — `Check::Processor`, both parts
  (ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section 6.2.10.4.1).
- `signatures/signature-object-conforms-to-pkcs7` — `Check::Unchecked`, ISO 19005-2 Annex B.1
  only, split out of `signatures/signature-is-a-single-signer-cms-object`'s reason.

None of the three is a predicate a document can fail, so none enters
`pdf_transform::archive::unconsidered()` and
`crates/pdf-transform/tests/archive_unconsidered.txt` is unaffected and unedited.

Two existing rows' reasons were rewritten without changing their identifiers:
`signatures/digest-covers-the-whole-file` and
`signatures/signature-is-a-single-signer-cms-object`.

## The figures, off the run

The corpus harness (`tools/bounded.sh --data 12 --tree 12 -- cargo test --profile gates -p
pdf-archive --test corpus -- --ignored`), exit 0, peak 0.12 GiB over the process tree:

| target | agreed | missed | over | settled | elsewhere | unreadable |
|---|---|---|---|---|---|---|
| PDF/A-4 | 473 | 0 | 0 | 8 | 6 | 0 |
| PDF/A-4f | 9 | 0 | 0 | 0 | 2 | 0 |
| PDF/A-4e | 17 | 0 | 0 | 1 | 1 | 0 |
| PDF/A-2b | 971 | 1 | 0 | 7 | 7 | 0 |
| PDF/A-2u | 21 | 0 | 0 | 1 | 0 | 0 |
| PDF/A-2a | 27 | 0 | 0 | 0 | 0 | 0 |

`over` is 0 on all six, before and after. The one miss is the standing
`6-6-2-3-3-t03-fail-b`, where errata A029 makes our answer the right one.

The coverage census (`cargo run -q -p pdf-archive --example targets`):

| target | binds | checked | unchecked | processor | clarified |
|---|---|---|---|---|---|
| PDF/A-2b | 180 | 125 | 18 | 36 | 1 |
| PDF/A-2u | 182 | 127 | 18 | 36 | 1 |
| PDF/A-2a | 191 | 134 | 20 | 36 | 1 |
| PDF/A-4 | 167 | 118 | 18 | 31 | 0 |
| PDF/A-4f | 167 | 117 | 18 | 32 | 0 |
| PDF/A-4e | 170 | 116 | 18 | 36 | 0 |

Nothing moved out of or into `checked`: the whole growth is the two new `Processor` rows on every
target and the one new `Unchecked` row on the three part 2 targets.

The structural audit (`cargo run -q -p pdf-archive --example frontier`): 220 subclauses listed,
**111 of them read sentence by sentence** against 52 before, **350 normative sentences** against
128 — 269 carried by a row, 26 restated at another clause, 28 scoping, 27 stating no requirement.
**65 subclauses have no sentence-level reading**, against 124 before, and the example lists them
by name.

## Gates

Green, all of them: `cargo fmt --all --check`, `RUSTFLAGS="-D warnings" cargo clippy --workspace
--all-targets`, both `fuzz/` lines, `cargo test --workspace --doc`, `cargo test -p conformance`
(6 passed on `tests/conformance.rs` plus every other binary), the corpus harness at exit 0, and
`cargo nextest run --workspace --no-fail-fast` at **4343 run, 4343 passed, 35 skipped**.

**Two failures seen mid-session were a sibling round's and are gone**, recorded because the shape
recurs in a shared working tree: `every_quotation_is_the_standards_own_words` reported
`tools/pdfref/src/undrawn.rs` and then `crates/pdf-syntax/src/write.rs:19`, both files this round
does not own and both open in another round's hands at the time. Re-running after that round
finished is the whole fix, and it is `doc/todo/02` section 2's own instruction about a build error
naming a crate you may not touch, applied to a gate.

## Files touched

- `crates/pdf-archive/src/coverage.rs`
- `crates/pdf-archive/src/survey.rs`
- `crates/pdf-archive/src/table/fonts.rs`
- `crates/pdf-archive/src/table/graphics.rs`
- `crates/pdf-archive/src/table/interaction.rs`
- `crates/pdf-archive/src/table/file_structure.rs`
- `doc/adr/0986-the-second-prefix-and-the-subclause-numbers-that-changed-between-editions.md` — new
- this file

## What the next round should know

- **The frontier is clause 6.3 onwards in both parts**, and `--example frontier` lists it by
  name. Two places to expect the sentence count to exceed the row count, offered as a ranking to
  check rather than to trust: ISO 19005-2 section 6.6.2.3.3, where one row stands for four tables
  of required fields, and ISO 19005-4's signature subclauses, whose section 6.5.4 is recorded as
  stating no requirement at all — the *expensive* kind of verdict ADR 0981 section 3.1 named, a
  claim about every sentence of a subclause made at a granularity that cannot see one. It was
  spot-read this round and the verdict holds: all three of its sentences are `should`. That
  reading is deliberately **not** in `READINGS`, because one scattered entry would cost the
  region the prefix shape that makes it reviewable against a copy — which is the whole reason
  ADR 0981 chose a prefix. The next round takes it in order with the rest of clause 6.5.
- **A `Processor` row costs nothing and an `Implemented` row costs a second crate's commit.**
  That asymmetry has now shaped three rounds' output, and it is worth saying out loud that it is a
  property of the *ratchet* rather than of the standard: `crates/pdf-transform/tests/
  archive_unconsidered.txt` is held to equality in both directions, so the round that writes a
  predicate owes the converter a decision in the same commit. A round that owns both crates can
  close `signatures/digest-covers-the-whole-file` — after settling the Annex B.1 reading ADR 0986
  section 3 names, which is the part that is not a scheduling problem.
- **The base-standard sweep is finished for `crates/pdf-archive` and is not finished for the
  tree.** Every `§` in this crate was resolved in both editions' heading lists; the same walk over
  `crates/pdf-model`, `crates/pdf-syntax` and `crates/pdf-font` has never been made, and those
  crates have no part-2 targets to make it matter — which is exactly the argument that would need
  checking rather than assuming.
- **One limit of the sentence-level instrument is now visible**: a row that binds a part through a
  *clarification* rather than through that part's own text has no sentence to hang on.
  `graphics/named-resources-are-defined` is the only one, and ADR 0986 section 5 says what the
  second such row should buy.
