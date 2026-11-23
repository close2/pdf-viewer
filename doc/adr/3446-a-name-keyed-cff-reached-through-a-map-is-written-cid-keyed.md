# 3446 — A name-keyed CFF reached through a CIDToGIDMap is written CID-keyed

Session 2186. Status: **accepted** and built; ADR 3428 item 2. Context: ISO 32000-2 §9.7.4.2,
Table 115, §9.9's Table 124; ISO 19005-2 section 6.2.11.2, ISO 19005-4 section 6.2.10.2; Adobe
Technical Note #5176 sections 13, 18 and 19.

## Reading

ADR 3428 item 2's: `issue7901`'s `CIDFontType0` holds a name-keyed CFF inside `/OpenType` and a
`/CIDToGIDMap` stream. §9.7.4.2 uses the CIDs "directly as GID values" for such a program, Table
115 gives the map to Type 2, and this reader draws through the map because without it the page
reads as nonsense. veraPDF reads the clause's glyphs and fails 6.2.11.5 at every CID.

## Decision

1. New row `fonts/cidfonttype0-glyphs-selected-by-cid` (6.2.11.2 / 6.2.10.2, conformance to the
   base standard's font clauses): a `CIDFontType0` whose CFF, bare or wrapped, has no `CIDFont`
   operators and which states a `/CIDToGIDMap` stream.
2. `pdf_font::cff::cid_keyed_by_map` writes the CID-keyed program: glyph 0 at CID 0, and for each
   CID the map sends to a glyph other than 0 that glyph's charstring, byte for byte, at the next
   index, its CID in a format 0 charset; one Font DICT naming the source's Private DICT (entries
   in their own bytes, `Subrs` re-pointed), the Local and Global Subr INDEXes unchanged in count
   and so in bias, `FDSelect` format 3, `ROS` from the `CIDFont`'s `/CIDSystemInfo` as text, the
   source's `FontMatrix` and `FontBBox` kept. It returns nothing that does not read back: the
   charset sends each CID to its glyph, units per em match, and every CID draws the same outline
   bit for bit at the same charstring width.
3. `Rewrite::CidKeyedProgramWritten` files it as `/FontFile3 /Subtype /CIDFontType0C` and removes
   the map. **Not back inside the `OpenType` wrapper** ADR 3428 sketched: the wrapper's `maxp`,
   `hmtx` and `cmap` count and name the old glyphs, and a wrapper whose tables disagree with its
   `CFF ` table is a program that does not conform to its own format; Table 124 files a bare
   CID-keyed CFF under a `CIDFontType0` directly.
4. Refused by name: a program two `CIDFont`s share, a map or program that does not decode, a
   `/CIDSystemInfo` without a registry or an ordering, a map past the program's glyphs.
