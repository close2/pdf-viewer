# 2273 — A Type 3 font's maps are restated as read, and every XML reader answers for a reference

Lane `gwg`, PDF/A: the census burn-down's leftovers (records 2259–2270). ADRs 3793 to 3796.

- **Type 3** (ADR 3793): `type3::encoding` logs a name outside Table 109's three as
  `font/encoding-name-unknown`. The validator read no Type 3 font (`load_every_font` skipped them),
  so their readings reached the log only where something else interpreted the page; it reads them
  now. `archive::font_maps` restates a Type 3 font: a name read as no table becomes `<< /Type
  /Encoding /Differences [] >>` (Table 110 requires a dictionary), a `/BaseEncoding` so named goes,
  a `/Differences` code past one byte is read past; proved by `Type3Font::read` naming every code
  alike (it was `true` unproved before). Shared encoding dictionaries restated two ways are refused.
- **One route** (ADR 3794): a font written directly inside another object is restated in its holder
  (`sites::fonts_inside_edited`); the placeholders route takes them out of the map as read
  (`font_maps::map_as_read`), so the font names one stream, whole and free of placeholders.
- **`unescape`'s answer** (ADR 3795): rich text returns `model/rich-text-malformed`, the popup's
  `/RC` walk logs it (and a walk that did not parse whole); XFDF owes it on `owed`; XFA logs the new
  class `model/xfa-reference-undefined` (section 5.1). CDATA is no longer unescaped in the rich text
  and popup walks (XML 1.0 section 2.7). `xmp::unescape` is removed.
- **TrueType instructions** (ADR 3796): the loader can tell, so `LoadedFont::load` starts the
  interpreter of a hint-reliant program and logs `font/truetype-instructions-unstarted` (section
  5.1). A per-glyph instruction failure stays silent: no document in the outline path.
- **`/Annots`**: thirteen `pdf-transform` readers go through `pdf_model::page_annots`; `linked_popups`
  (refuses a non-array by name), `merge` (another document's value) and the writers are left.
- Rebased onto 2272's merge (`3f09762dd`); `repair_census` passes on the new lines.
- **Census**, against 2270's (`r2270/`), with the pack and without: every profile line and every
  per-reason document list as 2270's — no document of the corpora reaches a route this round
  changed, so none moved either way. veraPDF 1.30.3 over the 5523 outputs: 5471 compliant, 40 not,
  12 unfinished, its log byte for byte 2270's.
- `tools/state.sh --round 2273 launch`: 6 documents, 36 figures banded, 0 outside.
- Ledger: §9.6.4, §9.10.3, §12.7.4.3, §12.7.3 and §9.9 stay `implemented`, gaining code and tests.
- Left, in order: a per-glyph TrueType instruction failure (no document at the outline); a draw
  proof for `cmap-damaged` (ADR 3782); bit 26's rich text value (`appearance::rich_text_value`)
  keeps a reference unlogged, holding no document; the report rendering `Restated::entries`.

**Gates.** Over `-p pdf-syntax -p pdf-font -p pdf-model -p pdf-archive -p pdf-transform -p
viewer-qt -p conformance`, after the rebase: fmt 0, clippy `-D warnings --all-targets` 0,
`pdf-sandbox --bins` 0, nextest `--lib --bins --tests` (`xvfb-run -a`) 6358 of 6358; rustdoc of
the four library crates 0; fuzz `cargo check --bins` 0; `cargo test -p conformance` 0. Before the
rebase `conformance::batch the_wrappers_last_line_names_the_seconds_gates_reads` failed once under
load (3 s for 1–2) and passed alone.
