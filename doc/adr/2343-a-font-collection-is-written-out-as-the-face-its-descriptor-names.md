# 2343 — A font collection is written out as the face its descriptor names

Session 1754, the archive slot of batch one hundred and fifteen. Status: implemented. Amends ADR
2327 section 1, which left a `ttcf` refused with this lever named. Context: ADR 0141 (this reader
draws one face of a collection); `doc/third-party-data.md` for the two texts read online.

## The construction

Table 124's `FontFile2` row asks a TrueType program as the TrueType Reference Manual describes one —
a single font opening with its own directory — and a collection is several (ADR 2327). This reader
already draws one face: the one whose PostScript name the descriptor's `/FontName` states, subset tag
and hyphens aside (ADR 0141). `pdf_font::embedding::collection_face` writes that face out as a program
of its own: its tables byte for byte and no other face's, under a directory written afresh — sorted
by tag, search fields, every table's checksum and `head`'s `checkSumAdjustment` stated, as the
Manual's font directory and Microsoft's OpenType font file chapter both give them, the latter adding
that a collection does not use `checkSumAdjustment`, so the face's own value means nothing once it
stands alone. `archive::font_files` files the face by its own format, as any program is filed, and
writes the stream whole with its `/Length1`. The face's tables are not completed: what the source's
face lacks, the output's lacks, and veraPDF is the check that it was a font.

**Face zero is not written.** ADR 0141 draws face zero where no face carries the name, and records
it as a fallback: the document did not choose it. Written into an archive it would become the
file's statement, so `pdf_font::collection::extract_named` answers only a named face, `extract`'s
behaviour is unchanged (the same expression, the search factored out), and such a collection — or
one program whose dictionaries name two faces — is refused by the sentence ADR 2327 wrote, reworded.

## What it moved

`issue13193` (`CambriaMath`, face 1 of 2) and `issue9262_reduced` (`MS-Mincho`, face 0 of 2) convert at
2b and 4 under both shipped profiles; page 1 is byte-identical to the source's at 100 dpi and 360,
looked at; veraPDF 1.30.3 passes the four outputs. The two faces of each collection share their
outlines, so `archive_corpus`'s witness test holds the written program table by table to the face the
reader chose, and a planted face zero fails it on `issue13193`.
