# 3412 — The veraPDF failures left standing after session 2177

Session 2177. Status: **accepted** (readings; each remedy named and not built). Context: ADR 3376,
`doc/pdfa-blocking-issues.md` section 1.

1. **`issue7901`, 6.2.11.5 and 6.2.10.5.** A `CIDFontType0` whose `/FontFile3 /OpenType` wraps a
   name-keyed CFF, with a `/CIDToGIDMap` stream. `pdf_font::composite::cid_to_glyph` reads the map
   (Table 115 defines its meaning without condition; ignoring it draws nonsense), so the program's
   advances agree with `/W` through it. veraPDF takes §9.7.4.2's sentence that such a program uses
   the CIDs directly as glyph indices, and finds fifteen widths that disagree. Both are readings of
   the standard; the remedy that satisfies both is to re-key the program so that each CID is its
   glyph index (charstrings and `hmtx` reordered by the map) and drop the map. Not built.
2. **`bug920426`, 6.2.11.4.1 and 6.2.11.8.** Its Encoding CMap maps codes with `beginbfchar`, the
   operator of a ToUnicode CMap (§9.10.3), where an encoding CMap maps codes to CIDs (§9.7.5). This
   reader takes the destinations as CIDs; veraPDF reads no mapping and finds CID 0 shown. The remedy
   is the embedded-CMap route (`archive::fonts::embedded_cmap`) restating the map with
   `begincidchar` as read. Not built.
3. **`bug1068432`, `bug1308536`, 6.2.11.4.1.** A CFF real with no digit nibble (Top DICT and
   Private DICT respectively); veraPDF's parser refuses the program, this reader's reads the
   number. The remedy is a DICT restatement writing the value this reader drew with, offsets
   recomputed. Not built.
4. **`bug866395`, 6.2.11.4.1.** veraPDF's CFF parser reaches the end of the data; this reader draws
   the program. Undetermined until the program is walked against Technical Note 5176.
5. **The exemption over-reaches where a form states no `/Resources`** (found by ADR 3410's first
   census, pdf.js `issue6541`). `reach::owners` takes only objects stating `/Resources`, so a form
   stating none, which draws with the page's resources, contributes no names, and the page's
   entries it draws are exempted as unreferenced. ADR 3410's removal now reads every content
   stream and keeps them; the exemption itself should count such a form's names toward the
   dictionary it draws from. Not built.
