# 3493 — A code whose name no route reaches is given the empty glyph, under an OpenType CFF face too

Session 2198. Status: **accepted**; built. Context: ADR 3475 (the build named), ADR 3125 (the empty
glyph in a font made symbolic), ADR 2458 (the empty glyph named outside the Adobe Glyph List under
a CFF face), ADR 2880 (a substitute's code the file takes to no glyph), ADR 2870 and ADR 3326 (an
unreadable program drawn from, and restated over, the substitute's face), `doc/questions/A48`.
Clauses: ISO 32000-2 §9.6.5.1, §9.6.5.2, §9.6.5.4, §9.10.2; ISO 19005-2 sections 6.2.11.4.1,
6.2.11.5, 6.2.11.8; ISO 19005-4 sections 6.2.10.4.1, 6.2.10.9.

## What the window drew, and what is written

`issue13316_reduced`'s `/YouYuan` embeds a `/FontFile2` that fails its Adler-32, so the window draws
it from a substitute (ADR 2870), here this machine's `NimbusSans-Regular`, an OpenType program with
`CFF ` outlines. Its `/Differences` name codes 0x41 and 0x43 to 0x46 `/g5167` and the like: no
character under the Adobe Glyph List and no glyph of the face's `post`, so
`LoadedFont::unreached_in_substitute` answers them and the window draws nothing for them (ADR 2880).
That is a statement about the file — §9.6.5.4's routes from a name end there on any machine — so it
is the population ADR 3475 named.

1. **`face_widths` gives such a code the empty glyph**, beside the code the encoding sends to
   `.notdef`, where the empty glyph is offered. The proof is the existing one: `empty::named_proved`
   reads the written font back and holds every given code to a glyph drawing nothing and every
   other shown code to the glyph and advance the window drew.
2. **An OpenType face with `CFF ` outlines takes it as a bare CFF face does.** Restated under a
   `Type1` dictionary (ADR 2773), it reaches glyphs by name (§9.6.5.2), so the glyphs are named
   outside the Adobe Glyph List (ADR 2458). `pdf_font::embedding::with_empty_cff_glyphs` appends
   them to the `CFF ` table and carries the sfnt with it: `maxp` counts them, `hmtx` states their
   advances as long metrics for every glyph (`hhea`), `vmtx` gains one at its last advance height
   where the face is vertical. Every glyph held keeps its index, name, charstring and metrics. This
   also lifts the refusal for a code on `.notdef` under such a face, which the census measures.
3. **A48's condition, read for this population.** The glyph this conversion gives is named outside
   the list and so states no character of its own. The file's `/ToUnicode` maps 0x41 to U+5F00; it
   is the producer's statement, kept byte for byte, and the text a reader extracts is the source's.
   Removing it would be a loss of the producer's text, not a condition A48 sets.
4. **No new loss.** The output draws what the window drew — nothing — for those codes; the font's
   substitution is reported as it was. The report and `xmpMM:History` name the codes in a clause
   of their own (`fonts::empty_glyphs_given`), because "reached .notdef" is not true of them.

## Proof

Under `as-shown` and `as-if-printed`, at 2b and 4, `issue13316_reduced` converts; veraPDF 1.30.3
passes all four outputs, and page 1 of each `as-shown` output renders at 100 dpi to the PNG the
source renders to, byte for byte. Tests: `crates/pdf-font/tests/wrapped_cff_empty_glyphs.rs`.
