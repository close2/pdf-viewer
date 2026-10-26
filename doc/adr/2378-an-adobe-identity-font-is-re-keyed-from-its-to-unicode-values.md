# 2378 — An Adobe-Identity font the file does not embed is re-keyed from its ToUnicode values

Status: implemented. Session 1772, the archive slot of batch one hundred and seventeen. Builds the
first of owner answer A570's two remedies; the second, the rasterized page, is the next archive
slot's. Context: `doc/questions/Q570`, `A570`, `A569`, `A566`; ADR 0270, ADR 2162, ADR 2302.

## The reading

§9.7.5.2: "The Identity-H and Identity-V CMaps shall not be used with a non-embedded font." Under
`Adobe-Identity` a CID indexes the glyphs of the program that defined it (§9.7.4.2), and that program
is not in the file. The viewer draws such a page all the same, code → `/ToUnicode` character → a
face's glyph (ADR 0270, §9.10.2's first method). A570: the converter writes that selection down,
names it as a loss, and invents nothing. The output repairs an invalidity: once a program is
embedded the identity `CMap` is legal.

## The decision

`archive::fonts::rekeyed`, reached from `composite::embed` where no `--font` program was named and
the descendant is not a collection the pack answers: an `Adobe-Identity` descendant whose `Type0`
font states a `/ToUnicode` stream, under `Identity-H`.

1. **Each shown CID's character is the producer's map's**, one character, not a value ISO 19005-2
   section 6.2.11.7.2 names unusable (the row `fonts/to-unicode-values-are-usable`'s own test);
   anything else refuses by name.
2. **The face**: the shipped face for the descendant's request (`pdf_font::standard::face`, ADR
   2162's) where it has a glyph for every character; otherwise the CJK pack's program for the first
   collection whose `Uni*-UTF32-H` `CMap` reaches every character. Which collection is first is a
   choice the file cannot make: Table 122's `/Lang`, then kana (Japan1) or Hangul (KR), then
   §9.7.5.2's own order, CNS1, GB1, Japan1, KR.
3. **The glyphs are numbered by the document's CIDs.** A `glyf` face is embedded whole under
   `/FontFile2` with Table 115's `/CIDToGIDMap` stream sending each CID to its character's glyph; a
   `CFF ` face is a §9.9.2 subset whose charset gives each glyph the document's CID
   (`pdf_font::embed::renumbered`, one charstring per CID), under `CIDFontType0C`. The content
   stream, `/Encoding`, `/W`, `/DW` and `/ToUnicode` are untouched; advances are restated and proved.
4. **Two losses, carried by the one decision at `fonts/font-programs-embedded`**: `machine-face`,
   and the new `glyphs-from-to-unicode` — the glyph choice is inferred from an extraction aid.
   `as-if-printed` discards both through its `discard` of that site; `as-shown` stops.

## The departure from A570's letter, said out loud

A570 says "the shipped collection face". The population this tree holds is not CJK: of pdf.js's 22
`Adobe-Identity` refusals the fonts name Latin faces (Arial, Calibri, Times New Roman, Minion Pro)
and one Arabic, and the five that state a `/ToUnicode` show Latin, Cyrillic or Polish text. Embedding Source Han Sans's Latin for a Times New Roman would draw a sans where the
window draws a serif. The base shipped face comes first because it is the face ADR 2162 embeds for
the same name and the one closest to what the window shows; the collection face answers what it
lacks, as A570 asks. Both are this program's shipped faces and both are the same two losses.

## Consequences

- issue2840, issue9084 (Liberation Sans, `/CIDToGIDMap`) and issue20065 (Foxit serif, renumbered
  CFF) convert at 2b and 4 under `as-if-printed`, veraPDF 1.30.3 PASS at both.
- Refused still, each by its sentence: issue5244 (Polish letters neither the shipped serif bold nor
  a pack collection reaches), issue5801 (a map stating none of its codes); fonts with no
  `/ToUnicode` keep `COMPOSITE_NOT_SUBSTITUTED` and are the rasterized page's population.
- Refused by sentence and buildable when a document asks: `Identity-V`, a face change between
  pages of one descriptor, one `glyf` glyph two CIDs show at two widths.
