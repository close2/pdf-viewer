# 2520 — A rendered TrueType font whose codes reach glyphs by a route the part withdraws is made the symbolic font the specification describes, with a `cmap` stating the glyphs this reader drew

Session 1843, an "archive" round of batch 125, 2026-10-28.
Status: implemented. R03a (the `.notdef` rows) is **owed**, not built here: section 7.
Context: `doc/questions/A694` item 3 (the owner's ruling, under A48's line: state an interpretation
the standard defines, never fill in an absence), `doc/questions/Q694`, `doc/pdfa-blocking-issues.md`
R11, ADR 0816 (the fence this ends for four rows), ADR 1942 and ADR 1943 (the `Differences` route for
the fifth row, which this does not touch), ADR 0947 (nothing is changed that no failed requirement
asked for).

## 1. What was refused, and why it was refused

ISO 19005-2 section 6.2.11.6 and ISO 19005-4 section 6.2.10.6 hold a rendered TrueType font's
embedded program and encoding to ISO 32000-2 §9.6.5.4, and four of their sentences fail on the
real producers' files of R11: a non-symbolic font's `Differences` names a glyph outside the Adobe
Glyph List; the program lacks the Microsoft Unicode (3, 1) subtable the `Differences` needs; the
program has no `cmap` subtable through which every needed code reaches a glyph; a symbolic font's
program lacks the subtable its part names. In every one, a code reaches its glyph in this reader by
a mapping §9.6.5.4's last sentence leaves to the processor ("a PDF processor may supply a mapping of
its choosing"), which ISO 19005 withdraws. ADR 0816 refused all four by name because the repair
edits the program's `cmap`, a table of the producer's file, and the sentence it gave said that the
edit "invents outlines" and "moves the marks".

**Does the operation invent marks?** CLAUDE.md's line for what this program may write asks that
question of every operation on a producer's content, and the answer here is no. No outline is added:
`pdf_font::embedding::with_tables` replaces the one table `cmap` and every other table, `glyf`
included, is the producer's bytes in the producer's order. No mark moves *for this reader*: the proof
in section 3 loads the font again and finds every one of the 256 codes on the glyph it reached
before, and the render check in section 6 finds the pages byte-identical or within one grey level of
the output intent's rounding. What the edit changes is which route reaches the glyph, from a mapping
§9.6.5.4 leaves to the processor to the route the same clause states; a *different* reader that chose
another mapping drew another glyph before and draws this one now, and that is the cost section 4
names. The clause itself describes the construction in its own list of guidelines (doc/md
`ISO_32000-2_sponsored_EC3.md` line 7906): a font used for glyphs that do not use the two standard
encodings

> should not specify an Encoding entry. The font descriptor's Symbolic flag (see "Table 121 -Font flags") should be set

with a (1, 0) subtable in the program's `cmap` and an optional (3, 0) one, and NOTE 1 says a program
"could need modification to conform to these guidelines". The edit is what the clause asks a program
to be, not marks the program never drew.

## 2. The decision

A694 item 3 ruled that the converter bakes into the PDF a conforming representation of the state the
viewer currently shows. For these four rows that is the route §9.6.5.4 states without a name
anywhere:

> When the font has no Encoding entry, or the font descriptor's Symbolic flag is set (in which case
> the Encoding entry is ignored), this shall occur:

and a (3, 0) subtable is then read with the high byte of one of its ranges prepended to each byte of
the string; failing that a (1, 0) subtable is read by the single byte. So `Rewrite::TrueTypeCmapRestated`
(name `truetype-cmap-restated`, module `crates/pdf-transform/src/archive/fonts/truetype_cmap.rs`)
makes the font the symbolic font §9.6.5.4 describes:

- the glyph each of the 256 codes reached in the font as this reader loads it is read out
  (`LoadedFont::glyph_index`), the reader's own choice included;
- the program's `cmap` table is replaced by a Macintosh Roman (1, 0) subtable of format 6 mapping
  each code to its glyph and a Microsoft Symbol (3, 0) subtable of format 4 mapping 0xF000 plus the
  code to the same glyph; every other table of the program is the producer's bytes, in the same
  order (`pdf_font::embedding::with_tables`). §9.6.5.4's NOTE 1 is the licence: "a TrueType/OpenType
  font program to be used in a PDF file could need modification to conform to these guidelines";
- a copy of the descriptor states Symbolic and not Nonsymbolic (Table 121 bits 3 and 6) and names the
  new program; the font dictionary names the copy and states no `/Encoding`;
- no content-stream byte changes.

The four rows are `Remedy` entries `Answer::Stated(None, Rewrite::TrueTypeCmapRestated,
TRUETYPE_CMAP_REINTERPRETS)` and no longer `REFUSED_BY_NAME`. They need no authorisation under
either shipped profile because nothing is lost; the report states the interpretation.

**The three neighbouring rows take it as their second answer.** `fonts/truetype-codes-reach-glyphs-by-the-standard-route`,
`fonts/symbolic-truetype-states-no-encoding` and `fonts/non-symbolic-truetype-uses-a-standard-encoding`
have narrower rewrites of the dictionary's `/Encoding`, each proved only for the codes the pages
showed. They are now `Answer::OrCmapRestated(&inner)`: the inner answer where the document can take
it, because it changes less (a name in the dictionary rather than a program and a descriptor), and
the restatement where the inner preparation refuses and the restatement's own proof holds. The
preparation restates the fonts those rows name only for a row that failed and whose own preparation
refused. The brief's own case, a code the file names no character for (no `/ToUnicode`, names with no
`post` entry), is the first of the two tests in `tests/archive/truetype_cmap.rs` that replaced the two
refusals which had held it.

**The file's own history names the clause (`doc/questions/A48`).** A694 item 3 places R11 under A48's
line, whose condition is a record in `xmpMM:History` naming the clause. `CmapRestated::history`
writes one entry of the action `converted`: the clauses ISO 19005-2 section 6.2.11.6 and ISO 19005-4
section 6.2.10.6, what the program now carries, that no mark moved and no content stream changed, and
the font objects (the first 20, then a count). It is a condition of the rewrite: a packet that will
not take the entry withdraws the restatement (`NO_PLACE_TO_RECORD_CMAP`), as for a transfer function.

## 3. The proof, and what makes a font refuse

**Nothing is written that was not proved.** The font is loaded again from the dictionary, descriptor
and program exactly as they will be written, from objects held in memory, and each of the 256 codes
has to reach the glyph it reached and must not need a mapping of the reader's choosing
(`LoadedFont::glyph_needed_a_reader_chosen_mapping`). The font keeps the refusal, with the
`NOT_RESTATED` sentence, where it is not an embedded TrueType program under a descriptor of its own,
where the sfnt cannot be re-packed, where it fails the proof, or where the checker named only a
prefix of the places a requirement fails (section 5).

**A code that reached glyph 0 is left out of the table.** A `cmap` lookup answering glyph 0 answers
that the character has no glyph, so no subtable states the route to it; after the rewrite such a
code is held only to not reaching another glyph. The requirement that owns a code on `.notdef` is
`fonts/no-notdef-glyph-shown` (ISO 19005-2 section 6.2.11.8), and R03a is the work that would answer
it: owed, untouched here.

**The copy is made from the program the conversion writes.** The metric restatement
(`fonts/widths-agree-with-the-program`, `Rewrite::RestateFontMetrics`) rewrites the original program
object; a copy of the unrestated bytes would leave the new program disagreeing with the dictionary's
widths, and the output would fail the checker. The copy is therefore made from the restated bytes
where the conversion restates them (the `restated_programs` map built from the metric
restatement); the first test that converted a font whose widths also disagreed found this.

## 4. Cost

- **What the file asserts.** `TRUETYPE_CMAP_REINTERPRETS` says it in the report: the font is now the
  symbolic font §9.6.5.4 describes, no glyph changes for this reader, a reader that chose another
  mapping drew another glyph before and draws this one now, and the encoding's glyph names no longer
  say which character a code is, so the text a reader extracts comes from `/ToUnicode` where the font
  has one. A requirement about the Unicode mapping (ISO 19005-2 section 6.2.11.7) is judged on the
  output like every other, and a font the conversion leaves failing it makes the whole conversion
  refuse, as always.
- **Size.** A new program and a new descriptor replace the old two, which nothing references and
  the rewrite of the file does not write, so the output holds one program and one descriptor per
  font: 94 115 bytes against the source's 91 343 for the test font, the difference being the table
  that was rewritten and the history entry.
- **Not a mapping change for this reader.** The page is the same glyphs; the render check below
  measures it.

## 5. The bound

**Three bounds, because the count of fonts alone admits any size.** (1) The checker keeps the first
32 places of each requirement and counts the rest (`pdf_archive::Findings`), so the fonts named are
at most 32 a requirement; a requirement cut short leaves fonts unnamed the output would still fail on,
so it refuses with the sentence "the checker named only the first places" instead of by the
conversion's last check (test `a_file_with_the_most_fonts_the_checker_names_converts_and_one_more_is_refused`,
32 fonts sharing a 140 KB program convert and 33 refuse). (2) **A budget in bytes**,
`MAX_COPIED_BYTES` = 32 MiB of decoded program summed over the fonts restated, because each font
carries a program of its own: 32 fonts sharing one 17 MB program peaked at 429 MiB and 119 s where
the unrestated conversion peaked at 140 MiB and 57 s. Past the budget the row keeps its refusal with
its own sentence ("pass the 32 MiB of font program one conversion copies, so none is copied"), and
the same 32-font file now ends in that refusal after 37 s and a 147 MiB peak (dev profile of the
export, `quorra-transform archive`, `as-shown` 2b); one such font converts (peak 141 MiB, 3.7 s).
The test `programs_past_the_byte_budget_are_refused_with_their_own_sentence` pads the test font to
1.2 MiB: 20 fonts convert, 30 refuse. 32 MiB admits one 17 MB face twice, which is what a CJK face
shared by a few fonts needs, and is a choice, not a derivation. (3) **No copy for a refused file.**
Where another failed requirement is one the table refuses by name, the document is refused whatever
its fonts say, and the preparation is skipped with `CMAP_NOT_PREPARED` instead of copying every
program first; a photographed page (`Loss::PageAsImage`) can still answer such a row, so the skip
does not apply when that loss is authorised (test `the_restatement_is_not_made_for_a_file_another_row_refuses`).
Nothing loops or recurses over a font's bytes other than the 256 codes and the table builders, which
are bounded by 256 entries and by the 16-bit fields of the `cmap` formats (`checked_*` arithmetic
throughout, the format 4 builder returning `None` rather than truncating).

## 6. What it frees (measured, dev profile of the export, with the CJK pack unless stated)

The 32 documents R11 names, one converter run each, four arms, HEAD's tree beside this round's:

| arm | converted, HEAD | converted, this round | veraPDF at the arm's own target on this round's |
|---|---|---|---|
| `as-shown` 2b | 2 | 14 | 13 pass, 1 fail |
| `as-if-printed` 2b | 2 | 14 | 13 pass, 1 fail |
| `as-shown` part 4 | 4 | 14 | 13 pass, 1 fail |
| `as-if-printed` part 4 | 4 | 14 | 13 pass, 1 fail |

The one veraPDF failure on each is `pdf.js:issue6961.pdf`, section 8. The corpus census
(`doc/pdfa-blocking-issues.md` section 1, `archive_corpus`, the pack present; "converted" counts the
failing documents the converter's own checker passes, pdf.js then corpora): `as-if-printed` 2b 838 to
848 and 386 to 388; `as-if-printed` 4 841 to 849 and 385 to 387; `as-shown` 2b 797 to 807 and 333 to
335; `as-shown` 4 801 to 809 and 331 to 333. `issue6961` is among the pdf.js gains and does not
conform (section 8), so counted as not conforming the pdf.js figures are 847, 848, 806 and 808. R11
as `rank.py` counts it: 30 documents (19 alone) to 13 (3 alone) under `as-if-printed` at 2b, 28 (17) to
13 (3) at part 4, 30 (17) to 13 (1) and 28 (15) to 13 (1) under `as-shown`; the 13 are
`truetype-codes-reach-glyphs-by-the-standard-route` 5, `symbolic-truetype-states-no-encoding` 5 and
`non-symbolic-truetype-uses-a-standard-encoding` 3, whose inner rewrite refuses and whose restatement
could not be proved or was not prepared (the reason per document is not traced here). HEAD without
the pack reads 827 and 386, 830 and 385, 797 and 333, 801 and 331 on the same four lines, so the
pack accounts for 11 pdf.js documents under `as-if-printed` and none under `as-shown`; this round's
tree was not re-run without the pack after the second answer was added.

**The page is the same**: for the 12 documents the first build converted, 20 pages rendered with
`quorra-transform render` from the source and from the output, 16 are byte-identical and 4 differ in
18 to 1 506 of about 2.18 million pixels by at most 1 of 255, the rounding of the output intent a
PDF/A conversion adds to the device colours; the glyph shapes do not move. The two documents the
second answer added were not rendered; the proof that every code reaches its glyph holds for them
as for the rest.

## 7. What is not done

- **The three neighbouring rows' own proofs** are unchanged and refuse where they fail; the
  restatement answers those rows as their second answer (section 2), so what is left of R11 is a
  font the restatement itself cannot prove, a file past the byte budget, or a file refused for
  another reason (the recount is in section 6).
- **R03a** (the `.notdef` rows, A694 item 3's second half) is owed.

## 8. Found, not fixed

`pdf.js:issue6961.pdf` converts, passes this tree's checker and fails veraPDF 1.30.3 at ISO 19005-2
6.2.11.4.1 (test 2), 6.2.11.6 (test 2) and 6.2.11.5 (test 1). The cause is not this rewrite: the
output still holds five fonts whose `Differences` name glyphs `uniXXXX`, which this tree's checker
passes and so leaves as the producer wrote them. `pdf-archive`'s
`non_symbolic_truetype_differences_are_listed_names` (`crates/pdf-archive/src/table/fonts.rs`, the
`encoding::text_for` call) accepts `uniXXXX`, the `_` ligature form and a `.suffix`, which are the
Adobe Glyph List *Specification's* algorithm for deriving a name's Unicode value. The clause asks
for that the names be ones the Adobe Glyph List itself lists, the list ISO 19005-2 cites normatively (the 2002
`glyphlist.txt`), in which `uni0042` is no row: the algorithm derives and does not list, so the
text of the part is the narrower reading and veraPDF follows it (its message says "Differences are
Unicode compliant = false"). **This tree's checker is too lenient there**, and it is a defect of
`pdf-archive`'s, to be fixed in a `pdf-archive` round (the gates `cross_check` and the corpus would
move); it is recorded in `doc/pdfa-blocking-issues.md` section 8. Until then a document with such
names converts and does not conform, and the census lines count `issue6961` as converted although it
is not conforming: with it counted as not conforming, each pdf.js gain this round reports is one
fewer.
