# 1307 — A CFF machine face subset, and described in memory as what it is

The ledger/writer slot of batch forty-seven. ADRs 1449, 1450. Rows: §9.9.1, §9.9.2 (notes and tests;
statuses stay `implemented`).

**CFF subsetting** (ADR 1449). `pdf_font::embed::cff` writes §9.9.2's subset of a `CFF ` face as a
bare CID-keyed program under `/FontFile3 /Subtype /CIDFontType0C`. That is Table 124's row for a
"Type 0 CIDFont program represented in the Compact Font Format (CFF)". The `OpenType` wrapper would owe
a rebuilt `cmap`, and §9.9.1 already gives a reader without `fsType` the narrowest use. The charset
gives each kept glyph its old index as its CID, so the stream is unchanged and §9.7.4.2's charset route
reaches every glyph. `embed::reach` runs each kept charstring with one stack across calls and hintmask
bytes skipped, and records every `callsubr` and `callgsubr` target after TN #5177's bias. Unreached
subroutines become a one-byte `endchar`, so each INDEX keeps the count its bias depends on. Font DICTs
are kept per kept glyph, and `FDSelect` is rewritten. Offsets are five-byte integers, laid out in two
passes. The Name INDEX carries the tagged name, as Table 117's `/BaseFont` requires. A face whose
licence forbids subsetting is still written whole under `OpenType` (ADR 1438).

Sizes: Noto Sans Duployan Bold (the saved fixture, seven glyphs) goes from 608 614 bytes of `CFF ` to
4 125, and the saved file from 252 423 to 5 108 bytes. All 77 `CFF ` faces on the machine, subset to
every glyph and to three glyphs, draw every kept outline identically to the whole face through
`pdf_font::cff::draw`. The six `CFF2` variable faces are refused as before.

**In memory** (ADR 1450). `machine_font` now describes a `CFF ` face as a `CIDFontType0` over
`/FontFile3 /OpenType` with no `/CIDToGIDMap`. Glyph indices and outlines are identical to the
`CIDFontType2` description. The fixture's screen raster hash is `a55f473ca05ea843` before and after. A
CID-keyed face whose charset renumbers a displayed glyph is no longer used, because it would draw
another glyph under either description.

Tests: `embed::cff::tests` cover closure through local and global subroutines and through each other,
hintmask data byte 29, and identical outlines with a plant (a closure missing a reached global
subroutine draws differently). They also cover a two-FD CID-keyed face with `FDSelect`, the tag and the
Name INDEX, licence bits, and the machine faces. `machine_cff_face_written.rs` checks `CIDFontType0C`,
the tag, the subset under a tenth of the face, `qpdf --check`, and the raster equal to the screen.

Left: a face using the accented `endchar` is written whole. A CID-keyed face that renumbers is not
drawn (its CIDs are not shown in the stream).
