# 1288 — A right-to-left value, joined and ordered

Batch forty-four. ADRs 1413, 1414. No question written.

**Contract.** §12.7.4.3's variable text in a right-to-left script: `freetext_no_appearance.pdf`,
the one incomplete corpus document that was this reader's, and ADR 0348's three parts.

**Done.**
- `pdf_font::shaping` (new): UAX #9 by `unicode-bidi` (already in the lock under `stringprep`),
  rule L1 per laid-out line in-tree, L2 by the crate, L4 from `BidiMirroring.txt`; the Unicode
  Standard's cursive joining (section 9.2, R1–R7 and ligature rules L1–L3) over
  `DerivedJoiningType.txt` and `ArabicShaping.txt`, each form reached as the presentation-form
  code point `UnicodeData.txt` decomposes to it. Five UCD 18.0.0 files in `data/unicode/`,
  compiled by `pdf-font/build.rs`; `BidiCharacterTest.txt` read by a test.
- `variable_text`: a value the compiled-in fourteen cannot draw is set in a machine face covering
  every character the shaped value displays (`installed_covering`, ADR 1382's precedent), written
  as a `Type0`/`CIDFontType2` with `/W` and `/ToUnicode`; each line is displayed in UAX #9's order,
  with caret, click and selection mapped through it; `/Q` read as sides of the box.
- A machine-face appearance is drawn and never written into a file (§7.3.8.1): the three save
  paths and `appearance::for_annotation` treat it as owed.
- `freetext_no_appearance.pdf` draws whole with no report and leaves `INCOMPLETE`.

**Measured.** `BidiCharacterTest.txt`: every line agrees through `pdf_font::shaping`. The Arabic
fixture's glyph sequence (worked by hand from the tables) fails when reordering is planted out;
the save test fails when its guard is planted out. `raster_golden`: `freetext_no_appearance.pdf`
moved (raster, list, reports) and nothing else of this round's moved.

**Left.** Executing a face's `GSUB` (faces with no presentation forms, a document's own such font,
which is reported as `FormsNotInFont`); writing a machine face into a file properly (indirect,
subset, embedding permission read); GPOS mark placement (marks overhang by the face's own metrics).
