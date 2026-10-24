# 2307 — A font program is filed by its format, and refiled from its bytes

Session 1736, the archive slot of batch one hundred and thirteen. Status: implemented. Amends ADR
0941's residue paragraph. Context: `doc/history/1730` ("Found, not fixed": five part 4 outputs veraPDF
fails at 6.2.10.4.1, read there as a `CIDFontType0` whose `FontFile3` states no `/Subtype`);
`doc/todo/72`, which names the five.

## 1. What the five were

The premise did not hold. Over pdf.js's and `doc/corpora`'s files every `/FontFile3` states a
`/Subtype` (a census of the stream dictionaries, calibrated by listing every subtype it found).
veraPDF 1.30.3's first test for the clause is `containsFontFile`, and it fails where a program is
filed under a key or name its bytes are not: `bug921409` (a bare CFF named `Type1C` under
`CIDFontType0`), `issue5751` (a bare CFF under `/FontFile`, the Type 1 key, under `CIDFontType0`),
`issue9949` (a `glyf` program named `CIDFontType0C` under an inline `CIDFontType0`), and
`issue13193` and `issue9262_reduced` (a `ttcf` collection under `/FontFile2`). The same census found
three more of the first kind the corpus run had not converted to part 4 before (`issue6782`,
`issue7598`, an `OTTO` program named `CIDFontType0C` or `Type1C`; `issue4668`, a `glyf` program
named `Type1C` under `TrueType`, which veraPDF passes). Each is the producer's, carried byte for byte.

## 2. The reading

ISO 19005-2 section 6.2.11.4.1 and ISO 19005-4 section 6.2.10.4.1 require the programs embedded *as
defined in* ISO 32000's section 9.9. ISO 32000-2's Table 124 "summarises the ways in which font
programs shall be embedded in a PDF file, depending on the representation of the font program",
filing each format under a key by the dictionaries that may carry it, and its Table 125 (Table 126 in
ISO 32000-1) makes the name of a `/FontFile3` required:

> A name specifying the format of the embedded font program. The name shall be Type1C for Type 1
> compact fonts, CIDFontType0C for Type 0 compact CIDFonts, or OpenType for OpenType fonts.

So a `/FontFile3` stating no name or one outside the three, a program under a key or name its bytes
contradict, and a pairing no row admits under the font's dictionary are not embedded as section 9.9
defines. ADR 0941 said no clause of either part is about a `/FontFile3` subtype; the embedding
clause's "as defined in" is, and the residue moves here from `conformance/adheres-to-the-base-standard`.
What a program must be beyond what the table asks of it stays section 6.2.11.2's (`Unchecked`).
**A `ttcf` collection is left unsettled**: whether it is "a TrueType font program, as described in
the TrueType Reference Manual" is the Manual's to say, it has not been read here, and the two
documents stay veraPDF's disagreement until it is (`doc/todo/72`).

## 3. Built

`fonts/font-programs-filed-by-their-format`, both parts, over the rendered fonts, one finding per
stream. `pdf_archive::table_124_rows` is the table asked of a dictionary subtype and the bytes: the
rows that admit them, none, or unsettled (no format this reader names, a `CFF ` Top DICT it cannot
read), which says nothing rather than guess. A Type 1 program is the one opening with `%!`. The
converter's `Rewrite::FontProgramRefiled` is `Mechanical`: a row under the same key restates the
name alone; failing that a `/FontFile3` row moves the stream there and drops the three lengths a
compact program does not state. The program is carried, and this reader already chooses its reader
by the bytes (`pdf_font::program`): page 1 of the five that convert is byte-identical, source against
output, at 100 dpi. Refused by sentence: a pairing no row admits (`issue9949` — a true filing needs
the dictionary changed to `CIDFontType2`, and with it how a CID selects a glyph, §9.7.4.2), a move
into `/FontFile` or `/FontFile2` (lengths this conversion does not compute), and a shortened finding
list. veraPDF passes the five at 4 and at 2b — evidence about this reading, not its source.

**What it costs the corpus**: four pdf.js documents that converted before are refused by the first
of those sentences, at both targets and under both profiles: `issue9949`; `issue11740_reduced`, a
Type 1 program under a `CIDFontType0` dictionary, which veraPDF fails too; and `bug1186827` and
`issue5084`, a CFF-flavoured `OTTO` program under `/FontFile2` in a `CIDFontType2` dictionary, as
`issue215` (refused already by another row) files one in a `TrueType` one. Table 124 admits a CFF
program only under `Type1` and `CIDFontType0`, and veraPDF passes the two — a disagreement taken to the table, which states the pairing outright.
For a `CIDFontType2` whose `/CIDToGIDMap` is `Identity` over a name-keyed CFF the glyphs a CID selects
would not change under `CIDFontType0` (§9.7.4.2 uses the CIDs "directly as GID values"), so that
refiling is provable and owed (`doc/todo/72`); for a simple `TrueType` dictionary the routes differ
(§9.6.5.4 against a CFF's names) and it is not.
`pdf_transform`'s `key_for` reads the same rows for a face it writes; making it ask `table_124_rows`
is left to a refactoring round.

## 4. ISO 19005-2 section 6.2.11.3.1, for the record

veraPDF's test for the clause requires `CIDFontSupplement <= CMapSupplement`. The clause says the
CIDFont's supplement shall be greater than or equal to the CMap's, and its note gives the reason (the
font has glyphs for every CID the CMap can reach). This validator reads the text; the three outputs
veraPDF fails there (`issue11555`, `noembed-eucjp`, `noembed-jis7`, round 1734) are veraPDF's
disagreement, written into the row's entry in `doc/pdf-a-mitigations.md`.
