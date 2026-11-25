# 2230 — A repair made without a document is returned to the reader that holds one

Lane `gwg`, the silent-repair campaign's census burn-down, second pass (ADRs 3574, 3597). ADR 3621.

- **The decoders.** `pdf_syntax::text_string_noting` returns `syntax/text-string-undecodable` and
  `Document::text_string` logs it; seven readers of a file's text (outline title, page label
  prefix, optional content name, annotation and structure text, media text) read through it.
- **A `/ToUnicode` lone surrogate stays held**: marking the map damaged moved nine documents from
  2b to 4 (one identity map over the surrogate block); Q1051 asks, with a remedy.
- **Declines, not repairs.** `sfnt::horizontal_metric` answers `None` past `hmtx`'s end, and its
  writers and converters decline as their documents said; `filter::decode` and
  `decode_with_parms` answer whole streams only (one LZW test now reads the reported decode).
- **Classes**: `syntax/header-missing` (§7.5.2, also 6.1.2's row), `model/blend-mode-unknown`
  (§11.6.3), `model/resource-category-not-a-dictionary` (§7.8.3). `appearance.rs`: 31 of 32
  sites, a Sonnet helper's work read back here (its `/DA` check logged an empty string and was
  corrected); `forms_data.rs`'s FDF repairs are said in the import's `owed`.
- **Markers read**: `xref.rs`'s `sections`, `walk_revisions` and `/Size` read the chain `read` logs;
  `composite_cycle` asks and supplies nothing.
- Left: `with_stand_in_font` (no document), the other `text_string` callers, the image unpackers
  (round 2228's files this batch), and a rewrite of an unknown `/BM` as `/Normal` as a remedy.
- `HELD` 571 → 515.
- **Census**, with the pack, against round 2224's: one document moves. pdf.js `bug911034`, whose
  `hmtx` holds all 8 320 pairs and none of the 42 057 bearings after them, is refused by name in
  every profile and part where it was converted with zero bearings; its advances are stated, so
  the remedy is to take each missing bearing from `glyf` and is the next round's. The converter's
  sentence for it still speaks of a Type 1 charstring. The census without the pack ran past the
  wall budget, queued behind another round's clock run, and finished at the merge (exit 0);
  veraPDF over the outputs is owed to the post-merge train.
- `tools/state.sh --round 2230 launch`: 6 documents, 36 figures banded, 0 outside.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-font -p pdf-archive -p pdf-transform -p
viewer-qt -p conformance`: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox --bins` 0,
nextest `--lib --bins --tests` 6180 of 6181 (`batch::the_wrappers_last_line_names_the_seconds_gates_reads`
failed beside the census's locked walk and passes alone; the manifest is empty), rustdoc 0, fuzz 0;
`cargo test -p conformance` 0.
