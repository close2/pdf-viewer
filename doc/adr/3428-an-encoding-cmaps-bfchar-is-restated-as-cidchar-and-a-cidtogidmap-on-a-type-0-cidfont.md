# 3428 — An Encoding CMap's bfchar is restated as cidchar; a CIDToGIDMap on a Type 0 CIDFont

Session 2181. Status: **accepted**; item 1 built, item 2 a reading with its construction named and
not built. Context: ISO 32000-2 §9.7.4.1 Table 115, §9.7.4.2, §9.7.5.4, §9.7.6.2; ADR 3412 items 1
and 2.

1. **`bug920426` (built).** §9.7.5.4 c): "The beginbfchar and endbfchar shall not appear in a CMap
   that is used as the Encoding entry of a Type 0 font; however, they may appear in the definition
   of a ToUnicode CMap." The font's Encoding CMap maps 1317 codes in 14 `beginbfchar` sections.
   The meaning is recoverable: §9.7.6.2's algorithm looks codes up in "the mappings defined by
   beginbfchar … and corresponding operators for ranges", and `pdf_font::cmap` reads each
   destination hex string as the CID, most significant byte first. New row
   `fonts/encoding-cmap-maps-codes-to-cids` (6.2.11.3.3 / 6.2.10.3.3, the CMap embedded "as
   described in" §9.7.5); `Rewrite::EncodingCmapRestated` writes each section as `begincidchar`
   sections of at most a hundred entries, source codes as the same bytes, every other byte of the
   program unchanged, and refuses by name a destination that is no hex string of at most four bytes
   (a glyph name states no CID) or a CMap under which any code of its codespace selects another CID.
2. **`issue7901` (reading; not built).** Table 115's `CIDToGIDMap` cell reads "(Required for Type 2
   CIDFonts with embedded font programs)", and §9.7.4.2 states, for a CFF program whose Top DICT
   does not use CIDFont operators, "The CIDs shall be used directly as GID values". So for a
   `CIDFontType0` the clause decides the glyph and the map states nothing the clause asks for:
   veraPDF's reading is the text's. `pdf_font::composite::cid_to_glyph` honours the map all the
   same (its comment argues it), because ignoring it draws "üãÍ†Ë œÍ†ÿ¨ Ì{«" where the producer
   wrote "The Free Software Definition" — a departure for a malformed file, and the archive must not
   carry it. **The construction that makes both readings one**: rewrite the name-keyed CFF inside
   the `OpenType` wrapper as a CID-keyed one whose charset gives each glyph `map[c]` the CID `c`
   (a glyph two CIDs reach copied), so §9.7.4.2's first case selects through the charset exactly
   the glyphs the map selected; drop `/CIDToGIDMap`; restate the `CFF ` table's checksum and the
   head's adjustment. Proof owed: every CID the content shows draws the same outline at the same
   advance before and after. Not built: `pdf_font::cff` has no name-keyed to CID-keyed writer, and
   that writer is the work.
