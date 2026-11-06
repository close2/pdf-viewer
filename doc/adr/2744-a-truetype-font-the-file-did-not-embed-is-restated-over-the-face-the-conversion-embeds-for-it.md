# 2744 — A TrueType font the file did not embed is restated over the face the conversion embeds for it

Status: accepted; built in `crates/pdf-transform/src/archive/fonts/truetype_cmap.rs`. Session 1965,
a single round. Context: ADR 2520 (the restated `cmap`), ADR 2146 (a substituted font's descriptor
and dictionary), ADR 2458 (empty glyphs a substitution adds), `doc/questions/A694` item 3. Clauses:
ISO 32000-2 §9.6.5.4 and §9.9 (Table 124); ISO 19005-2 section 6.2.11.6 and ISO 19005-4 section
6.2.10.6, cited by section and paraphrased.

## What was refused

ADR 2520's restatement loaded each font the encoding rows named **as the file states it** and took no
font whose glyphs were a substitute (`sites::loaded_as_stated`). Eight of the ten documents R11 still
held stopped there, at the first font: a TrueType font the file never embedded, for which the
`as-if-printed` profile's substitution embeds a face this program ships. The output is then a
TrueType font with a program, and the rows the source failed are asked of that program; the
restatement, which could answer them, never looked at it. The other two (`507676`,
`issue13316_reduced`) stop on a `/FontFile2` whose RFC 1950 check value disagrees, which this reader
refuses to read (`pdf_font::program::whole_program`); no glyph was drawn from it, so there is no route
to state, and they keep the refusal.

## The decision

The restatement reads each font **as the conversion writes it**: the dictionary with the entries the
substitution adds, its descriptor with the program entry the substitution writes (`Embedding`), and
that program. Three cases follow, each from the clause.

1. **A font the substitution makes a `Type1` one** (the face shipped for it is CFF, filed under
   `/FontFile3` by Table 124) is no longer read by §9.6.5.4's TrueType route at all; it is passed
   over, nothing copied. `PDFBOX-3067-negativeTf` and one font of `issue6127` are this case.
2. **A font the substitution embeds a shipped face into** is restated from that face's **own code
   table** (`pdf_font::standard::shipped_face`), the table this reader draws the face by and the
   substitution proves its embedding with, each glyph held to having the same outline in the program
   embedded (which may be the face with its advances restated, ADR 2685's route). It is **not** taken
   from reading the embedded program back as a TrueType font: that reading is the very route the rows
   withdraw, and on `issue7580` it sends code 241 (`ae` by the base encoding) to another glyph whose
   advance is not the file's width, so the output failed 6.2.11.5 where the source drawn met it. A
   code the table leaves without a glyph keeps the one the embedded program gives it, which is the
   substitution's own (ADR 2458).
3. **Every other font** is ADR 2520's case, unchanged.

The proof is ADR 2520's: the font is loaded again with the new descriptor and program and every
code must reach the glyph restated, by no mapping of the reader's choosing; the output is held to
the target again. The restatement is the one ADR 2520 names (`truetype-cmap-restated`), stated in
the history as before; the face itself is the substitution's loss, stated by it.

## What it leaves

`sample_fonts_solidconvertor`: two fonts with an encoding dictionary the substitution photographs
the page around, so no program is embedded and nothing is restated. `SimFang-variant`,
`XiaoBiaoSong`, `issue20489`: refused by `file-structure/bound-names-are-valid-utf8` first, and the
first two also by a descriptor written inside the font dictionary.
