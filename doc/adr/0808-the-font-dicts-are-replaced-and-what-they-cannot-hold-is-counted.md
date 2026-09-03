# ADR 0808 — The unreadable Font DICTs are replaced, and what they cannot hold is counted: a CID-keyed CFF draws the glyphs under a Font DICT nobody can read against an empty Private DICT, and keeps the rest

Status: accepted. Session 880.
Clauses: ISO 32000-2 §9.7.4.2 (glyph selection in CIDFonts — the charset route), §9.9 Table 124
(`CIDFontType0C`: "shall conform to Adobe Technical Note #5176"), §9.7.4.3 (why the Private
DICT's widths do not matter here); Adobe Technical Note #5176 sections 5, 18 and 19 (INDEX, FDArray,
FDSelect) and #5177 section 4.1 (`callsubr`).
Code: `crates/pdf-font/src/cff.rs` (`FontDictRepair`, `readable_font_dicts`, `Layout`,
`calls_local_subr`), `crates/pdf-font/src/loading.rs` (`with_readable_font_dicts`,
`LoadedFont::program_shortfall`), `crates/pdf-model/src/content/font.rs` (the note).
Tests: `crates/pdf-font/src/cff.rs::an_unreadable_fd_array_is_replaced_and_the_glyphs_draw`,
`::a_readable_font_dict_is_kept_beside_the_replaced_one`,
`::a_glyph_that_needs_a_local_subroutine_is_counted_as_lost`,
`::a_program_whose_font_dicts_read_is_left_alone`; `doc/checks/fixed-documents.toml`'s row for
`batch5/FOP/FOP-2736-4.pdf`; `crates/pdf-model/tests/corpus.rs`'s `whose_defect` row.
Documents: §9.7.4.2's and §9.9's ledger rows, `doc/todo/03` §42.

## Context

`batch5/FOP` (808 documents, Apache FOP's issue tracker) surveyed at 34 incomplete, and the ink
ranking of those against `pdftoppm` and `mutool` had one head with both references agreeing:
`FOP-2736-4.pdf`, ours 0 against 4.35 and 4.36, reporting that `/F20`'s "program has no outline
for any of the 1816 code(s) the page shows through it". The page is FOP's glyph sheet of a
Japanese OpenType font, `EAAAAA+AoyagiSosekiFont2OTF`, embedded as a bare CFF under
`/FontFile3 /CIDFontType0C`, `Identity-H`, by Apache FOP 2.3.0-SNAPSHOT.

The program is CID-keyed (`ROS`), its charset is the identity over 7925 glyphs, and its
charstrings are whole. Its Top DICT puts the `FDArray` at offset 8001 — and its format-0
`FDSelect`, one byte per glyph, runs from 208 to 8133. The FDArray offset lands inside the
FDSelect's bytes, which are all zero, so every reader finds an INDEX of count 0 there:
`read-fonts` answers `InvalidIndexOffsetSize(0)` for every `subfont`, `fontTools` asserts,
and this tree drew nothing and said so. Both references draw the page because both rasterise CFF
through FreeType, which loads a CID font with no subfonts and interprets its charstrings against
the top-level defaults — two references agreeing through one library (trap 9), which is why the
argument below is from the formats and not from them.

Table 124 requires the program to "conform to Adobe Technical Note #5176", and this one does
not; the standard says nothing about what a processor does with one that does not. What it does
say is where the outline comes from: §9.7.4.2 reaches "the glyph procedure using the CharStrings
INDEX table", and that INDEX is intact.

The other shape of the same defect was already in `doc/pdf.js`, drawn and unreported:
`issue9278.pdf`, iText 2.0.8, two CID-keyed CFFs of 65 535 charstrings whose `FDArray` holds
nineteen Font DICTs of which the first four state no Private DICT at all — unreadable to
`read-fonts` — beside fifteen that read and carry the local subroutines their glyphs call. Glyphs
under the four drew nothing, silently, under the "blank glyph" count; glyphs under the fifteen
drew. It is the witness that decided the *shape* of the repair, below.

## Decision

**Every Font DICT a glyph selects that cannot be read is replaced by one with an empty Private
DICT; every Font DICT that reads is kept exactly as it is; and the glyphs under a replaced DICT
that needed the real one are counted, and reported by the page that shows them.**

The first draft replaced *all* the Font DICTs whenever one was unreadable, and the corpus gate
refused it in the first run: `issue9278.pdf` went from `complete` to reporting seven and eleven
lost glyphs, because the fifteen readable Font DICTs' subroutines had been thrown away with the
four unreadable ones — a glyph that drew before did not draw after. The gate's requirement that
every new sentence have a `whose_defect` row is what stopped it, and the second draft is the
one described here.

The argument that makes the replacement sound rather than a guess: a Type 2 charstring's
*outline* depends on its Private DICT in exactly one way. `callsubr` reaches the DICT's local
subroutines (#5177 section 4.1) and nothing else in the charstring does — `defaultWidthX` and
`nominalWidthX` decide the advance, which a CIDFont takes from `/W` and `/DW` under §9.7.4.3 and
never from the program, and the hint operators change no contour. So a charstring that never
calls a local subroutine draws, against an empty Private DICT, exactly the outline the producer
wrote; and one that does cannot be drawn at all, because the subroutine it names is not in the
file. `calls_local_subr` scans each charstring for `callsubr`, following `callgsubr` into the
global subroutines (whose INDEX is the Top DICT's and intact) with the operand stack tracked far
enough to know which one, and the hint-mask data bytes skipped by the stem count; a call whose
operand cannot be told counts as a loss, which errs on the side of reporting.

The repair is at the bytes, before `read-fonts` sees them — the shape ADR 0799 gave a JPEG's
`DNL`, and for the same reason: the reader is right to be strict about the form, and the way to
have it read what the program states is to state it in the form it reads. `read-fonts`'
`Subfont` cannot be constructed from outside, so a default one cannot be handed in. The Top DICT
is re-encoded with every offset operand in the five-byte form, so its length is known before the
offsets are; everything after the Top DICT INDEX is copied verbatim and shifted by the difference
— the Top DICT and the Font DICTs are the only places a CFF states an absolute offset, a Private
DICT's `Subrs` being relative to the DICT — and a fresh `FDArray` is appended, one entry per FD
the program's own held or a glyph selects: a kept Font DICT re-encoded with its Private offset
shifted, an unreadable one an empty Private DICT. The `FDSelect` stays the program's own. A
charset or Encoding operand at a predefined value is left as the table it names.

**What is reported, and what is not.** The loss is the *page's*, not the program's: a repaired
program none of whose lost glyphs the page shows has cost that page nothing, and a note making it
`incomplete` would be a report firing on a condition that costs the reader nothing (trap 11).
So `LoadedFont::glyph_lost_to_repair` is asked per code, on the arm where an outline came back
empty — before the "blank glyph" arm, which would otherwise read a glyph the repair could not
supply as the program describing it empty — and `text.rs` tallies it into the font's coverage
with the program's own sentence (`LoadedFont::repair_shortfall`); at the end of the page each
font with a loss says it once: *R of the N Font DICTs its CID-keyed CFF selects cannot be read
(FDArray at offset A), so the glyphs under them are drawn against an empty Private DICT, and the
L of its G glyphs that call a local subroutine that DICT cannot hold are not drawn; C of the
code(s) the page shows through it reach such a glyph*. The generic "no outline for any" report
stands down where every empty outline is explained by this one. `FOP-2491-1.pdf` is the lossy
witness in the same tracker — the same subsetter, both Font DICTs unreadable, 9 of 13 glyphs
calling local subroutines — and it went from "no outline for any of the 14 codes" to four glyphs
drawn and *12 of the code(s) the page shows through it reach such a glyph*.

## Consequences

- `FOP-2736-4.pdf` draws its glyph sheet at 4.38 by the ranking's instrument between the two
  references' 4.35 and 4.36, looked at glyph for glyph against `poppler`'s; the fixed-documents
  gate reads 8.759 on its own and that is the band. `batch5/FOP` is 33 incomplete.
- The repair is decided per program at load, once: `subfont_index` over every glyph and one
  `subfont` per distinct FD, which for a well-formed font is a handful of reads, and the
  charstring scan runs only on the glyphs under a replaced DICT of a program being repaired. The
  bytes are held once — the repaired copy becomes the font's `data` and the record keeps its
  figures and the lost glyphs' indices.
- `issue9278.pdf` stays `complete` in the corpus gate and its glyphs under the four unreadable
  Font DICTs draw where they drew nothing; the oracle's figures for it are in the round's
  history file.
- A CFF inside an `OpenType` wrapper is not repaired: it reaches its outlines through the `sfnt`
  route, and no witness of that shape has been seen. §9.7.4.2's row says so.
- A name-keyed CFF whose Private DICT's `Subrs` offset is wrong — `FOP-2751-3.pdf`'s
  `FuturaStd-Book`, `InvalidIndexOffsetSize(101)` at draw — is a different defect and is not
  this: nothing can be replaced there, because the subroutines the charstrings call are what is
  unreadable. `doc/todo/03` §42 names it as the tracker's next item.
