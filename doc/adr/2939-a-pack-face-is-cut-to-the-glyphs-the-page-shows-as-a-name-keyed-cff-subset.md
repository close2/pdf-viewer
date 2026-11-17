# 2939 — A pack face is cut to the glyphs the page shows, as a name-keyed CFF subset

Session 2062. Status: **implemented.** Context: ADR 2934 (the pack; round 2060's record left "subsetting the pack
face"), ADR 1449 (the CID-keyed CFF subset), ADR 2378 (the `Adobe-Identity` route), ISO 32000-2 §9.6.5.2, §9.9.1's
Table 124 and §9.9.2, ISO 19005-2 section 6.2.11.4.1, Adobe Technical Note #5176.

## 1. Why the pack's route did not subset

Two routes ask the pack. The `Adobe-Identity` route (ADR 2378) already wrote a subset: `issue5244`'s output names
`DVHNUQ+NotoSerif-Bold`, a CID-keyed CFF under `/CIDFontType0C`, 13 213 bytes in all. The descriptor route of a
simple font wrote the whole program — Noto Serif Regular is 677 184 bytes, about 244 kB once deflated — because
`pdf_font::embed`'s only CFF subsetter writes CID-keyed programs, which Table 124 admits under a `CIDFontType0` and
not under the `Type1` dictionary that reaches its glyphs by name. No route subsets any face for a simple font; the
shipped faces are small enough that nobody had asked.

## 2. The decision

1. **A name-keyed subset** (`pdf_font::embed::compact_subset`, `embed/type1c.rs`): the charstrings of the kept
   glyphs and `.notdef`, renumbered densely, every subroutine they reach (the CID subsetter's `Closure`, shared), a
   charset in format 0 giving each its own name, the Top DICT's descriptive entries and the Private DICT. §9.6.5.2
   reaches a simple font's glyph by the name its encoding gives a code, so renumbering moves nothing a reader looks
   up. A program whose built-in encoding is not the Standard one is written whole, since a custom encoding names
   glyph indices; the pack's faces state none.
2. **What is kept**: the glyph each shown code reaches in the font as written, read back with the whole face —
   ISO 19005-2 section 6.2.11.4.1 asks the glyphs the page renders, and §9.9.2 permits a subset.
3. **The name**, §9.9.2: "the value of the font's BaseFont entry and the font descriptor's FontName entry, shall
   begin with a tag followed by a plus sign (+) followed by the PostScript name of the font from which the subset was
   created". The tag is `pdf_font::embed`'s digest of the program and the kept glyphs, so two subsets in one file
   differ; the name goes into the program's Name INDEX too, since Table 109 makes a `Type1` font's `/BaseFont` the
   program's `FontName`. The dictionary's `Times-Roman` gives way to `XXXXXX+NotoSerif-Regular`: the program is
   Noto's, and the old name was already not the program's own.
4. **The proof** carries the whole face's across (`described::pack_face_subset`): read back with the new names,
   every shown code reaches the copy the subsetter gives its glyph, with the same outline command for command, and
   all 256 advances are still the source's. The OFL in the program's `/Metadata` is unchanged: condition 2 asks it
   of a modified copy as of the original.
5. A font the window drew as nothing keeps the whole face, whose shown glyphs are then emptied (ADR 2830).

## 3. Evidence

ADR 2934's fixture (`Times-Roman` and `Courier-Bold`, three codes each) under `as-if-printed`: 748 202 bytes at 2b
and 748 189 at 4 before, 13 070 and 13 057 after, each face cut to four glyphs. `issue5244`: 13 213 and 13 200
bytes, as before, its route having subset already. veraPDF 1.30.3 passes every output. The plant: the subset's
charset given each glyph its neighbour's name fails both pack tests, the proof refusing the font.
