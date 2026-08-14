# 515 — The hollow font that cannot collapse a band it never measures

Date: 2026-08-14. ADR 0350.

**The finding, in one sentence:** the OmniPage CSDK 22 failure the owner's brief describes —
a hollow embedded `CIDFontType2` (real `head`/`hmtx`, every `glyf` entry empty, a
`/CIDToGIDMap` remapping stream) collapsing another viewer's selection boxes to zero height —
cannot occur in this tree, because selection geometry takes its vertical extent from Table
120's `/Ascent`/`/Descent` through `measured_extent`'s band (ADR 0216) and never from the
glyph outlines; measured on a hand-built fixture pair in both CSDK shapes, extraction is
exact, every text-layer quadrilateral is the descriptor's 11.1 pt band to the hundredth of a
point, a headless drag selects the invisible word under full-height paintable quads, and the
blank-glyph accounting counts nothing for mode 3 (no mark owed) and all ten codes with a
named report for the visible control (ADR 0270's split working in both directions).

No production code changed — the round's product is the adversarial witness ADR 0216 never
had, since no corpus document combines a remapping stream with a hollow program.

Files touched:

- `crates/test-scenes/src/ocr.rs` (new) — `scanned_ocr_pdf(OcrFont, render_mode)`: the
  CSDK-22 and CSDK-20 shapes, hand-built per trap 8, with a by-hand hollow TrueType whose
  advances make `/W`-vs-`hmtx` agree only through the stream.
- `crates/test-scenes/src/lib.rs` — `assemble_pdf` extracted from `basic_pdf` and shared.
- `crates/pdf-model/tests/ocr_text_layer.rs` (new) — four tests: extraction from both shapes,
  the Table 120 band, the two-population accounting under modes 3 and 0, and §9.7.4.2's
  stream read against the identity.
- `crates/viewer-core/tests/headless.rs` — the drag across the invisible word, through the
  real boundary, asserting the selected text and the quads' height.
- `crates/viewer-core/Cargo.toml` — `test-scenes` as a dev-dependency.
- `crates/pdf-model/examples/hollow_glyph_census.rs` (new) — the command behind the trap-8
  claim: every corpus `CIDFontType2`'s own `loca` against its `/CIDToGIDMap`, printing the two
  populations and their intersection. The intersection is empty, and the claim had been an
  assertion in an ADR and a ledger row until it was run.
- `doc/conformance/ledger.toml` — §9.7.4.2's row gains the fixture witness and the stream test;
  §9.8.1's, the row that describes the band, gains the band test and the sentence saying what
  the fixture separates.
- `doc/adr/0350-…`, this file.

Not applicable, said rather than skipped: the selection-geometry instrument (ADR 0333) judges
the pdf.js corpus and cannot see a fixture outside it; `Query::Caret`/`Offset` are §12.7 form
questions and the fixture has no fields — text hit-testing is the pointer path, and
`select::position_at` has no distance threshold, so the razor-thin-hit-line failure has no
mechanism here at all. No drawing-path code changed — selection geometry is readback, not
drawing, and this round changed no production code of any kind — so the corpus and oracle
gates are not owed and `doc/todo/00` step 7 is not owed either.

**Gates, watched print**, after the last edit. `cargo fmt --all --check` clean. `cargo clippy
--workspace --all-targets` silent of lints (the `viewer-qt@0.1.0:` `-Wmaybe-uninitialized`
lines are the documented gcc output from the cxx bridge, not clippy's). `cargo nextest run
--workspace`: 1866 passed, 15 skipped — the five new tests are the whole difference from the
round before. `cargo test --workspace --doc`: every crate ok. `cargo test -p conformance --
--nocapture`: 5 checks ok. `cargo run --release -p pdf-model --example hollow_glyph_census`:
964 documents opened, 268 `CIDFontType2` dictionaries, 221 embedded programs read, 42 fonts in
30 documents through a `/CIDToGIDMap` stream, 214 programs with some empty glyph, 0 with only
empty glyphs, 0 in the intersection. The corpus, oracle, text, dates, XMP, JPEG 2000 and quorra
gates were not owed: nothing this round wrote is reachable from a raster or a readback of a
corpus document.
