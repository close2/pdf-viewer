# 2326 — A CIDFontType2 whose Identity map makes each CID its glyph, over a name-keyed CFF, is a CIDFontType0

Session 1746, the archive slot of batch one hundred and fourteen. Status: implemented. Amends ADR
2307 section 3, which left the refiling owed. Context: `doc/history/1736` ("Found, not fixed");
`doc/todo/72`.

## The reading

ADR 2307 refuses a CFF-flavoured OpenType program (`OTTO`) under `/FontFile2` in a `CIDFontType2`
dictionary: Table 124 admits a `CFF ` program only under `Type1` and `CIDFontType0`, so no filing
under the dictionary as written is true, and making one true changes how a CID selects a glyph.
For one shape the change selects nothing new. Under `CIDFontType2`, Table 115 makes a `/CIDToGIDMap`
named `Identity` the identity mapping between CIDs and glyph indices. Under `CIDFontType0`, §9.7.4.2
splits on the program's Top DICT, and for one that does not use CIDFont operators:

> The CIDs shall be used directly as GID values, and the glyph procedure shall be retrieved using
> the CharStrings INDEX

So where the map is the name `Identity` and the program is keyed by name, CID *c* selects glyph *c*
under both subtypes, and the dictionary is retyped: `/Subtype` becomes `CIDFontType0`, the
`/CIDToGIDMap` Table 115 gives a Type 2 `CIDFont` alone is removed, and the program moves to Table
124's `/FontFile3` `/OpenType` row as ADR 2307 already moves one. `/W`, `/DW`, `/W2`, `/DW2`,
`/CIDSystemInfo` and the `Type0` font's encoding are keyed by CID and read the same under both. It
is `Mechanical`, under the same rewrite (`FontProgramRefiled`), since nothing a reader draws moves;
this reader picks its glyph route by the program and not by `/Subtype` (the §9.7.4.2 ledger row), so
the page is drawn by the same code either way.

**Refused, by a sentence of its own**: a map stated as a stream, an absent map, or a CID-keyed
program, whose CIDs §9.7.4.2 would send directly or through the charset to glyphs nothing says they
selected before. Re-keying the `Type0` encoding and the widths to glyph indices would answer a
stream map; it is not built.

## What it moved

`issue5084` (map `Identity`, a name-keyed `Generic0-Regular`) converts at 4 and 2b under both shipped
profiles; page 1 is byte-identical to the source's at 100 dpi and at 360, looked at, and veraPDF
1.30.3 passes the four outputs. `bug1186827`, the brief's second witness, did not hold: its map is a
stream sending CID *c* to glyph *c* − 31 from CID 32, so it stays refused, now by the new sentence.
The census is the session's record (`doc/history/1746`).

What would reopen it: a re-keying of a `Type0` font's encoding and widths that a later round proves
selects the same glyphs.
