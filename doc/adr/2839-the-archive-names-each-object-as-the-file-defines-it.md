# 2839 — The archive names each object as the file defines it, and an index past the program is no glyph

Round 2010, the PDF/A lane.
Status: **accepted and built** in `pdf-archive` (`examination.rs`, `reach.rs`, `table/metadata.rs`,
`table/fonts.rs`).
Follows the owner's audit (round 2000), which made `pdf_syntax::Document::get` read a reference at a generation the
file does not define as null.
Clauses: ISO 32000-2 §7.3.10, §9.7.4.2; ISO 19005-2 sections 6.2.11.2, 6.2.11.4.1 and 6.2.11.5; ISO 19005-4
sections 6.2.10.2, 6.2.10.4.1.

## 1. One identifier per object

`pdf.js/test/pdfs/freetexts.pdf` converted under every profile at both parts before round 2000 and was refused by
every one after it, on `fonts/font-dictionary-states-type-font`, "written directly inside another object". The
premise that the file holds no such font is half true: the fonts its first revision names are objects of their
own, but its incremental update appends `1 2 obj`, a FreeText annotation the page's `/Annots` names, and
`2 2 obj`, its appearance stream, whose resources hold `/Invalid_font << /BaseFont /Helvetica /Subtype /Type1
/Encoding /WinAnsiEncoding >>` — a font with no `/Type`, written inside object 2 2.

The checker found it and named its holder `2 0`: `Examination::objects` keyed every object at generation zero,
and `reach` compared objects by number alone, both to agree with a `Document::get` that ignored the generation.
Round 2000 made `get` follow §7.3.10 — "the object number and generation number" together identify an object —
so `get(2 0)` is now null, and the converter, reading the holder the checker named, found no dictionary and
refused. Before round 2000 the same lookups read object 2 by number and typed the font.

§7.3.10 is the rule, so the identifier is the file's: `Examination::objects` keys each object at
`Document::defined_id`, `reach` keys an arrival at the reference's own identifier and finds an object only where
number and generation both agree, and `metadata_streams` reads each number at its defined generation. A finding,
a holder named to the converter and a reference the reach walk follows now all name the object the file wrote;
a reference that spells another generation reaches nothing, as §7.3.10 says it names nothing.

freetexts now converts under all eight profiles at both parts, every output passing veraPDF 1.30.3: writing
`/Type /Font` into the font inside 2 2 is ADR 2608's mechanical answer, a lossless restatement under every
profile.

## 2. A glyph index past the program's last glyph

ADR 2830 section 6 found a converted file whose `/CIDToGIDMap` sent CIDs to glyph indices past the subset's
last glyph: this tree's reader drew nothing, `fonts/embedded-programs-define-every-glyph-shown` passed it, and
veraPDF failed 6.2.11.4.1, 6.2.11.5 and 6.2.11.8. ISO 19005-2 section 6.2.11.4.1: embedded fonts "shall define all
glyphs referenced for rendering". A glyph index is a position in the program, so an index at or past the count
the program states (`maxp` for an sfnt, the `CharStrings` INDEX for a bare CFF) defines nothing. The row now
judges each shown code's glyph against that count, beside its existing judgement of glyph 0. A Type 1 program
states no count and is read by name, so it keeps the judgement it had.

Section 6.2.11.5 asks the dictionary's widths to be consistent with the program's, and the program states no
width for a glyph it does not hold: there is nothing to be consistent with, and the failure is section
6.2.11.4.1's, whatever a validator also prints under 6.2.11.5. Section 6.2.11.8's `.notdef` row is judged as
before: an index past the program is not glyph 0.
