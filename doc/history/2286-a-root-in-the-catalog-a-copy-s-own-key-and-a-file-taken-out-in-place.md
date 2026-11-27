# Round 2286 — a root in the catalog, a copy's own parent-tree key, and a file taken out in place

Lane 3. Subject: redaction's two leftovers from 2277 (§12.5.6.23, §14.7.5.4, A834) and the
archive converter's refusal of an inadmissible file written in place (2283). ADRs 3845–3847.

- **A structure tree root the catalog states directly** (ADR 3845): a `Place` carries
  `in_catalog`, the root's direct kids are walked and restated inside it, and the catalog is
  replaced and written with the restated root (`restated_dict`, `place_catalog`).
  `redaction_leaves_no_trace::a_structure_tree_root_the_catalog_states_directly_is_restated_there`.
- **A placement copy's `/StructParents`** (ADR 3846, `redact/parent_tree.rs`): each copy of a form
  stating one is keyed from `/ParentTreeNextKey` (or past the greatest key), its entry the form's
  array with null where the copy lost the sequence; pairs go at the end of the root's `/Nums` or
  into one new leaf after its `/Kids`. An element of the form's sequence now names every copy that
  still holds it, reached or not. `redact::a_placement_copy_is_given_a_parent_tree_entry_of_its_own`.
- **A file written in place** (ADR 3847): `pdf_archive::table::embedded_file_fails` judges one
  specification as the row does; `sites::InPlace` takes the failing ones out where they stand — an
  `/AF` element leaves the array, an annotation's own `/FS` loses `/EF` and `/RF` and keeps naming
  its file. Anywhere else still refuses. The 2283 refusal test became two removal tests and one under an action; veraPDF
  passes both outputs at 2b.
- Ledger: 12.5.6.23 stays `departed`, two tests added to it; 14.7.5.4 stays `implemented`.
- Left, in order: a shared form copied whole for one page (ADR 1196) still carries the form's key;
  a referenced specification a kept file attachment annotation names still refuses outside the
  files-as-pages profile, which ADR 3847's `/EF` removal would answer; a parent tree held directly
  in the root with `/Kids` keeps the form's key; an in-place specification under an action's `/F`.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-archive -p pdf-transform -p conformance`: fmt
0; clippy 101 (`map_unwrap_or_default`, `too_many_lines` twice, `single_match_else`), fixed, 0;
`pdf-sandbox` 0; nextest 5883 run, 1 failed, `records` on this paragraph unwritten (`audit-diff`:
no manifest entry, none expected); rustdoc of `pdf-transform`, `pdf-archive` 0; fuzz 0
(`pdf_archive` gained a function); `cargo test -p conformance` 101, the same `records`. Census
against 2283, pack and none: no document moves. veraPDF 5467 → 5471 compliant: the four `issue9949` outputs, 2283's own late fix, nothing of this round's. Launch 0: 6 documents, 36 figures
banded, 0 outside.

## Habit

A key the standard tells you to take from a counter is taken from the counter, and checked against
the tree it indexes: a stated `/ParentTreeNextKey` can be wrong.
