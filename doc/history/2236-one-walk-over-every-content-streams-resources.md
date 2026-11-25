# Round 2236 — one walk over every content stream's resources, and a text string that reads back

A fix round in the `gwg` worktree for audit 2234's three behaviour findings (§7.7 to §7.12), on the
audit's commit `b0f478b8` (fast-forwarded into the branch; nothing committed).

## What changed

- `pdf-model/src/resource_walk.rs` (new): `ResourceWalk` yields every resource dictionary §7.8.2's
  content streams name beneath a page or a mask's group — a form's, a tiling pattern's, a Type 3
  glyph description's own and then its font's (Issue #128's order), each annotation's normal
  appearance's on request — each indirect object once, no deeper than `content::MAX_FORM_DEPTH`
  (now `pub(crate)`), with `truncated()` when the depth cut. ADR 3645.
- B02: `optional_content::groups_referenced_by` and `groups_referenced_by_content` read
  `/Properties` and each `XObject`'s `/OC` in every dictionary the walk yields (the first with the
  annotations' appearances); their own form walk and its depth of 8 are gone.
- B01: `colourants::spot_colourants` reads colour spaces in every dictionary the walk yields; its
  `Resources`, `Font` and `Appearance` roles are gone. `content::resources_blend` reads `/BM` the
  same way, keeps its 256-dictionary guard and answers `true` at a cut.
- B03: `pdf_syntax::encode_text_string` takes PDFDocEncoding only where the bytes do not begin
  `FE FF` or `EF BB BF` (§7.9.2.2.1 NOTEs 3 and 4); every other text is written as before.
- The other pre-drawing walks (the PDF/A survey, image walk-ahead, `reach.rs`, `fenced.rs`, structure
  owners, the optimiser's image census, `parent_tree`, `default_rgb`) stay their own; ADR 3645 says
  why each differs. `pdf-transform/src/merge.rs`'s own label writer (ASCII or UTF-16BE) cannot
  begin with a prefix and is left as is.
- Audit tests pass; their four entries leave `expected-failures-current.json`; B01 to B03 marked
  `fixed` in 2234's `findings.json`.

## Ledger

Rows 7.8.3, 8.11.4.3, 10.8.3, 11.6.3 and 7.9.2.2.1 re-read; each stays `implemented`, with the
walk or the writer added to its note and tests. None of the three walks is on the launch path: spot
colourants under separation simulation, the blend walk for a luminosity mask, layers from the panel.

**Gates.** fmt 0; clippy `-D warnings --all-targets` of `-p pdf-syntax -p pdf-model -p pdf-colour -p
pdf-archive -p pdf-transform -p viewer-core -p viewer-qt` 101 on the new module (a verbatim quotation, a
`let`-`else`), fixed, then 0; `pdf-sandbox` 0; nextest `--lib --bins --tests` of the seven (`xvfb-run
-a`) 0, 6159 passed, `audit-diff.py`: "failed 0, expected 0"; rustdoc of `pdf-syntax` and
`pdf-model` 0; `cargo test -p conformance` 0 once this record stated its gates; `raster_golden`
held 974, moved 0. Fuzz not owed: no public type or signature changed.
