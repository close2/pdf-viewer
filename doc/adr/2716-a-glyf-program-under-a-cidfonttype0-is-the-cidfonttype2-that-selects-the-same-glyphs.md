# 2716 — A `glyf` program under a `CIDFontType0` is the `CIDFontType2` that selects the same glyphs

Session 1951, a single archive round.
Status: accepted and built. In `pdf-transform`: `archive/font_files.rs` (`rows_for`'s
`Retyping::ToType2`, `Refiled::to_type2` and `to_type2_within`, the refusal `NO_MAP_TO_KEEP`),
`archive/rewrite/values.rs` (`as_cid_font_type2`, written in the font or in its `Type0`'s
`/DescendantFonts`). Tests: `font_files::tests::a_glyf_program_under_cid_font_type0_is_the_cid_font_type2_selecting_the_same_glyphs`,
`…_mapped_by_another_name_keeps_its_refusal`, and `a_program_no_row_admits_keeps_its_refusal`
moved to the pairing that still has no row. Context: ADR 2307 (the refiling, and the refusal this
narrows), ADR 2326 (the opposite retyping), `doc/pdfa-blocking-issues.md` R10.

## The case

`fonts/font-programs-filed-by-their-format` (ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section
6.2.10.4.1) fails a TrueType program (`0x00010000`, a `glyf` table) filed under `/FontFile3`
`/CIDFontType0C` in a `CIDFontType0` descendant. ISO 32000-2 §9.9's Table 124 admits a `glyf`
program under a `TrueType` or `CIDFontType2` dictionary only, so no key or name under
`CIDFontType0` is true of it, and ADR 2307 refused it on the ground that a new subtype changes how
a CID selects a glyph. `pdf.js:issue9949` is the corpus's case.

## The reading

§9.7.4.2 states how a Type 0 `CIDFont` selects a glyph of a **CFF** program and says nothing of a
`glyf` one. This reader, given one, reads no charset (there is none to read) and selects through the
`/CIDToGIDMap` route (`pdf_font::composite::cid_to_glyph`): a stream as Table 115 describes it, the
name `Identity` as the identity, and an absent entry as the identity. That is exactly how a
`CIDFontType2` selects, by §9.7.4.1's Table 115:

> If the value of CIDToGIDMap is a name, it shall be Identity , indicating that the mapping
> between CIDs and glyph indices is the identity mapping.

So the subtype change selects no glyph the reader did not already select: it is ADR 2326's argument
run the other way.

## Decision

The descendant becomes `CIDFontType2`; a map it states is kept, an absent one is written as the
`/Identity` the reader read it as (which ISO 19005's `fonts/cid-to-gid-map-present` asks of an
embedded Type 2 `CIDFont` in any case); the program takes the `CIDFontType2` row under the key it is
already filed by — under `/FontFile3`, `/OpenType`, whose Table 124 row admits a `glyf` program under
`CIDFontType2`, so no length is computed and every byte is the producer's. A descendant written
directly in its `Type0`'s `/DescendantFonts` is retyped there. A map naming anything but `Identity`
is one this reader does not draw through, and keeps the refusal by its own sentence.

The rewrite stays `font-program-refiled`, a mechanical one: nothing the reader draws or extracts
moves, which the round proved by rendering each converted output beside its source.

## What stays refused

A CFF-flavoured OpenType program under a simple `TrueType` dictionary (`issue215`): Table 124 files it
under `Type1` or `CIDFontType0`, and a simple `TrueType` font selects through its `cmap` subtables
(§9.6.5.4) where a `Type1` selects by glyph name, so a retyping there would reinterpret the codes. A
Type 1 program under a `CIDFontType0` (`issue11740_reduced`) is filed under no row at all and would
need a conversion of the program to CFF.
