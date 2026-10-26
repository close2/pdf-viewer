# 2327 — A font collection is in no row of Table 124, and the writer asks the validator's rows

Session 1746, the archive slot of batch one hundred and fourteen. Status: implemented. Amends ADR
2307 section 2, which left a `ttcf` unsettled. Context: `doc/todo/72`; ADR 0141 (this reader draws
one face of a collection).

## 1. What a TrueType font program is

Table 124's `FontFile2` row asks a TrueType font program as the TrueType Reference Manual describes
one, and requires that the program conform to it. Nothing in this tree held the Manual; I read
Apple's published edition, chapter 6 (*Font Files*), from its website, and cite it by section and
paraphrase, never quote. Its font file is one font: the font directory comes first, it opens with an
offset subtable, and that subtable's first field is the scaler type naming the font's outlines —
`true` and 0x00010000 for TrueType, `typ1`, `OTTO` for CFF — with other values unsupported. A `ttcf`
header is none of them, and the Manual describes no collection anywhere. The `/FontFile3` `/OpenType`
row asks one OpenType font program whose tables its bullets name; Microsoft's OpenType specification
(its *OpenType font file* chapter, read as evidence for ISO/IEC 14496-22, which is not held) makes a
collection the delivery of several font resources, each with a table directory of its own, and a
font descriptor names none of them.

So a collection is in no row under any key. `pdf_archive::table_124_rows` answers it with no row
rather than leaving it unsettled, `fonts/font-programs-filed-by-their-format` fails it with a
sentence naming the collection, and the conversion refuses it with a sentence of its own.
`issue13193` and `issue9262_reduced`, which converted before into outputs veraPDF failed at
6.2.11.4.1 and 6.2.10.4.1, are refused at both targets under both profiles. What would answer them:
the face ADR 0141 already chooses by the descriptor's `/FontName`, written out of the collection as
a TrueType program of its own with its `/Length1`.

## 2. The writer's rows are the validator's

`pdf_transform::archive::fonts::key_for` read Table 124 a second time for a face the converter
writes. It now takes the first row `table_124_rows` admits, so the key a face is written under and
the key the output's second verdict asks of it are one reading. Over every compiled-in face, under
eight dictionary subtypes (320 pairings), the old reading and the new name the same row, and a
planted `last()` for `first()` failed that comparison. Two inputs no shipped face is would read
differently, an operator's `--font` alone could supply them, and each ends refused as before: an sfnt carrying both `glyf` and `CFF ` (the old reading wrote `FontFile2`,
which the second verdict refused; now the pairing is refused first) and a bare CFF whose Top DICT
this reader cannot read under `CIDFontType0` (the old reading refused it at once; now the row is
admitted and the face meets `proves`, which reads each glyph's advance from the program before it is
written and refuses one it cannot read).
