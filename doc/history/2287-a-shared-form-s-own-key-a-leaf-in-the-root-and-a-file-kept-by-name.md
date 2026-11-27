# Round 2287 — a shared form's own key, a leaf in the root, and a file kept by name

Lane 3. Subject: the three cases round 2286 left (ADRs 1196, 3846, 3847). ADRs 3849–3851.

- **A shared form copied for one page** (ADR 3849): the page's copy takes a parent-tree key and
  entry of its own as a placement's copy does, its reached sequences are the copy's, and an
  element names the copy by stream and `/MCID` whatever its `/Pg`. Writing it found a leak: a
  marked-content `/Stm` counted as a placement, so every tagged form read as shared, was copied,
  and the producer's stream, covered marks included, rode into the output on the structure tree.
  `/Stm` is no longer counted. `redact::a_shared_form_copied_for_one_page_is_given_a_parent_tree_entry_of_its_own`,
  `redact::a_tagged_form_one_page_draws_is_edited_in_its_own_place`.
- **A `/Kids` parent tree held in the root** (ADR 3850): the new leaf's number goes into the
  root's own `/ParentTree` `/Kids` once numbered (`Restated::parent_tree_kids`).
  `redact::a_parent_tree_of_kids_held_in_the_root_takes_the_copy_s_entry_in_a_new_leaf`.
- **A file kept by name** (ADR 3851): a specification something kept still names — an attachment
  annotation's `/FS`, an action's `/F`, a media clip — loses `/EF` and `/RF` and keeps naming its
  file, object or in place; a name-tree leaf's pair leaves the leaf. `EMBEDDED_FILE_STILL_REACHED`
  is gone. Three archive tests, the as-pages refusal test turned into a conversion; veraPDF 2b
  passes all six fixture outputs.
- Ledger: 12.5.6.23 stays `departed`, three tests added; 14.7.5.4 stays `implemented`.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-archive -p pdf-transform -p conformance`: fmt
0; clippy 101 (`too_many_lines` on `cut_traces`, `panic` in a test helper), fixed, 0; `pdf-sandbox`
0; nextest 5889 run, 5889 passed (`audit-diff`: 0 failed, 0 expected); rustdoc of
`pdf-transform` 0; fuzz 0; `cargo test -p conformance` 0. Census against 2286, pack and none: no
document moves. veraPDF over the pack's outputs 5471 of 5523 compliant, as 2286's. Launch 0: 6
documents, 36 figures banded, 0 outside.

## Habit

When a construction copies an object so the producer's stays, ask what still reaches the
producer's: a reference that draws nothing (a `/Stm`) carried the covered marks into the file.

Left: a form shared through an ancestor's or another page's resources is still carried whole
wherever those resources reach it, drawn or not (ADR 1196).
