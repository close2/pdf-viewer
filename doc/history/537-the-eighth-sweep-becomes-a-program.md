# 537 — The eighth sweep becomes a program, and the transform method nothing named

**Finding.** A sweep round under `doc/todo/01`'s binding rule. The pointer sweep — "does the file a
note names still exist?", run as a grep since the three-hundred-and-seventy-fifth — is
`conformance --bin pointers` now, and it reads the *symbol* half of a pointer as well as the path,
which `doc/todo/01` had named as a sweep somebody could build. **Its first run found three dead
pointers and all three are claims about tests.** `crates/viewer-host/src/policy.rs` justified
`resolve_import`'s purity by "which is what `tests/import_policy.rs` does" — a file that has never
existed in any commit of this tree, while the policy is tested one file over in
`tests/host_mappings.rs`; `crates/viewer-accessibility/tests/tree.rs` sent the bus half of its
subject to a `tests/atspi.rs` that was never written; and `doc/errata-read.md` quoted
`content.rs::alternate_image` for a function that moved into `content/image.rs` when the module was
split. The two path defects were both found by the program's one real decision — **a pointer is
resolved from where it is written**, so a `tests/x.rs` in a doc comment means its own crate's tests
— and a glob over the whole tree would have called both of them live.

**Second finding, from the blame band, and it is the sharpest.** §12.8.2's note said "`FieldMDP` and
`UR` are recognised where a `/Reference` names them". **Nothing in this tree names the string
`FieldMDP`**: `has_transform` takes a method name and `DocMDP` is its only caller, so a document
stating a FieldMDP transform reaches a reader as an ordinary signature. §12.8.2.4 was `partial` on
the strength of that recognition — the fourth failure shape, where the half of the note saying what
*is* done is the wrong half — and is `reported` now. The protection the clause exists for is not
missing from the program: §12.7.5.5's signature field lock, which Table 259 says a writer *copies*
into these parameters, is read and enforced by `viewer_core::notes`. It is simply not this transform
doing it.

**Third, the ninth sweep in a block.** Seven mis-attributed table numbers, six in `pdf-model` and
five of those in one `enum`'s doc comments, plus the tenth sweep's one: §14.8.2 counted twelve rows
below it over a family of thirteen, all thirteen present since the ledger was generated — written by
the five-hundred-and-first, a sweep round.

**Date.** 2026-08-15.
**ADR.** [0372](../adr/0372-the-eighth-sweep-becomes-a-program-and-the-transform-nothing-named.md).

**Sweep results, verbatim from the runs.**

- `pointers` (new): 4609 path pointers — 2525 live, 104 absent, 14 in another crate, 1616 unrooted,
  106 a form, 244 not carried; 48 symbol pointers, 9 undefined. **3 defects.** 84 of the 104 absent
  are in `doc/adr/` and in `doc/todo/01`'s own records of earlier runs. **The level moves when a
  round writes its own record** — this round's ADR and todo entry took the run to 4663/109 by
  narrating what it had just corrected, which is the sweep's own oldest false positive produced by
  the round that fixed it.
- Table numbers (9): 1074 *attributed* key citations against 408 tables, **7 defects**. Reading
  every key in every sentence that names a table instead was 545 hits to nothing, so the sweep
  counts a key only where the sentence attributes it to the table; a flags table has bit numbers
  rather than keys, and `doc/md/` puts Table 92's abbreviations in the second column.
- Parent counts (10): 70 checkable count claims against a family, 20 matching neither the direct
  children nor the descendants, **1 defect** (§14.8.2).
- `blockers`: ledger 21 — 6 expired, 10 holding, 5 naming no clause; source 27 — 10, 10, 7. 0
  defects.
- `capabilities`: ledger 49 — 34 witnessed, 42 program, 7 crate; source 145 — 119, 79, 66. 0
  defects.
- `unread`: 63 rows claim, 174 keys; 49 confirmed, 125 quoted over 53 rows, 55 by the row's own
  code. 0 defects.
- `entries`: 240 rows explain themselves by an arrival and name code, 1 names none; 640 entries
  stated, 112 reported over 45 rows — 29 named nowhere, 83 only elsewhere, 34 not named by the
  row's own note. Known populations.
- `quotations`: 3386 quotations in 533 documents, 1586 verbatim, 23 diverging, 0 defects.
- `callers`: **296** distinct `pub fn` names in `pdf-model` (289 in the five-hundred-and-twenty-fifth),
  15 crates naming it in a manifest — 176 named by a dependent crate, 20 by a tool or a fuzz target,
  77 only inside `pdf-model`, 21 only by a test or an example, **2 by nothing at all**; 120 that no
  crate under `crates/` asks. 0 defects: the two are the same two, both disposed of in their own doc
  comments by the round that found them.
- `retired`, over this wave's fourteen nouns: 1512 mentions, 2 nouns carrying both shapes, 0
  defects — 1462 of the 1512 are `Window` and `widen`, and `StreamSource` and `Agreement::Bounded`
  have none at all.
- Arithmetic (6): §7.9.2 and §O, clean. Inapplicable (7): 57 of 80 name source vocabulary on a
  session-local stop-list; none wrong, and exactly one row changed status since the
  five-hundred-and-twenty-fifth — this round's — checked with `git diff` over the status lines.
  Ledger quotation marks (11): 1270 spans, 913 verbatim, 78 diverging of which 22 hold no elision,
  0 defects. Sweep 14: 8 hits under a session-local vocabulary, every one naming its debt in other
  words. Errata (12): "151 struck passage(s) of 4 words or more that doc/md/ still carries as
  current text" over all fourteen PDFs, unchanged, and 75 quotations quoting text struck out of the
  clause they cite.

**Rows corrected in this commit.** §12.8.2.4 (**`reported`**, the recognition it claimed and does
not have, what the clause asks of a reader, and where the protection actually comes from), §12.8.2
(the parent's half-true sentence about the same mechanism), §14.8.2 (twelve rows below → thirteen).
Kept with evidence recorded: §12.7.4, §9.2.4, §12.7.6.2, §8.7.3, §11.7.5, §12.11, §7.7.2, §14.8.4,
§14.9, §12.8.2.2 — ten of the twelve the band held.

**Source corrected.** `crates/viewer-host/src/policy.rs` and
`crates/viewer-accessibility/tests/tree.rs` (the two tests that never existed),
`crates/pdf-model/examples/damaged_stream_census.rs` (Tables 126→125, 66→65, 78→77, 122→119),
`crates/pdf-model/src/view.rs` (Table 168→170 for a widget's `/N`),
`crates/pdf-model/src/viewer_preferences.rs` (Table 148→147 for `/Enforce`, twice),
`crates/pdf-model/tests/logical_structure_example.rs` (Table 354→37 for `/Nums`),
`doc/errata-read.md` (`content.rs`→`content/image.rs`).

**Code.** `tools/conformance/src/pointers.rs` and `src/bin/pointers.rs` (new, fourteen unit tests);
`tools/conformance/src/retired.rs` (`kind_of` and `paragraphs` shared with it);
`tools/conformance/src/lib.rs` (the module).

**Touched.** `doc/conformance/ledger.toml`, `doc/todo/01-ledger-partial-rows.md` (the run's record,
the band pointer advanced to commit 518, eight commands now),
`doc/todo/02-every-round.md` §4 (the new command's line), `doc/ledger-and-claims.md` (seven programs
→ eight), `doc/adr/0372-*` (new), this file.

**Gates.** `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets` silent of
lints; `cargo nextest run --workspace` **1975 tests run: 1975 passed, 15 skipped**;
`cargo test --workspace --doc` green; `cargo test -p conformance -- --nocapture` 5 passed. No corpus
or oracle run: every correction this round made is a comment or a ledger note, and `git diff` over
`crates/` shows no line outside a comment.
