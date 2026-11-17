# 2956 — Every slot of a name-keyed encoding names a glyph; a TrueType `cmap` is partial

Session 2071. Status: **accepted and built.** Supersedes ADR 0932 on one point: what an embedded Type 1
program's never-assigned code draws. ADR 0932's census and its `assigned_codes` stand. In `pdf-font`:
`LoadedFont::substitutes_notdef`, `name_keyed::simple_code_table`. Context: ADRs 0932, 2942, 2948, 0015. Clauses:
ISO 32000-2 §9.6.5.1 (Table 112), §9.6.5.2, §9.6.5.4, §9.2.4; the Adobe Type 1 Font Format, Version 1.1 (clause 2),
sections 2.3 and 10.3; Adobe Technical Note 5176 (CFF), section 12 and Appendix B; ISO/IEC 14496-22, `cmap`.

## Context

ADR 0932 read a code an embedded Type 1 program's `/Encoding` array never assigns as *unencoded*, and drew nothing
for it. ADR 2948 read an encoding's empty slot as naming `.notdef`, and advanced a name-alone standard font's code by
that glyph, on the ground that the Type 1 format fills every slot with `.notdef` first. ADR 2948 reported the clash
and left it. Both cannot be true of the same array.

## The texts

- Type 1 Font Format, section 2.3: "Type 1 BuildChar begins by using the character code as an index into the
  Encoding array in the font dictionary to obtain the name of the character to be built", for every code; the
  section's program builds the array as `/Encoding 256 array` `0 1 255 {1 index exch /.notdef put } for` and then
  the `dup <code> /<name> put` lines.
- Section 10.3, which is normative about the shape: "For special encodings, assignments must be performed as shown
  in the example in section 2.3". A PostScript array has no absent element, so a slot nothing assigned holds what
  the fill put there.
- §9.6.5.2: "All Type 1 font programs shall contain an actual glyph named .notdef. The effect produced by showing
  the .notdef glyph is at the discretion of the font designer."
- TN 5176, Appendix B: the predefined `StandardEncoding` lists codes 0 to 31 as SID 0, `.notdef`. §9.6.2.1 NOTE 1
  makes a CFF a "functionally equivalent representation of a Type 1 font program".
- §9.6.5.4, last sentence of the symbolic branch: "If a character cannot be mapped in any of the ways described
  previously, a PDF processor may supply a mapping of its choosing." ISO/IEC 14496-22's `cmap` lookups return
  glyph 0, "the missingGlyph", for a code a subtable does not hold.

## Decision

**ADR 2948's reading holds for every name-keyed encoding; ADR 0932's drawing does not.** Per case:

1. **An embedded program's own array.** A never-assigned code names `.notdef` and draws the designer's `.notdef`.
   It is drawn at the drawing step (`substitutes_notdef`), and the code table still says *no glyph of its own*,
   for ADR 0932's reason: three instruments read the table. A code whose readback is whitespace keeps the
   documented departure and draws nothing. The width is the file's `/Widths`, unchanged.
2. **`/Differences` over a base.** A dictionary naming no base layers over the program's array (Table 112), so
   case 1 applies to the codes it leaves. **A named encoding (`/Encoding /WinAnsiEncoding`, or a
   `/BaseEncoding`) is the whole mapping** ("shall override a Type 1 font's mapping from character codes to
   character names"). Its empty slot names `.notdef`; it no longer falls back to the program's own name for that
   code. A name the program lacks draws `.notdef`, as before.
3. **A standard-14 font named alone.** ADR 2948 stands: advanced by the drawn face's `.notdef`; the shipped
   faces' `.notdef` paints nothing.
4. **A TrueType symbolic font.** Unchanged. A `cmap` is a partial map: a code no route reaches is §9.6.5.4's
   processor's choice, and ADR 0015 makes that choice. The OFF's missing glyph is a lookup's *answer that there is
   no glyph*, not a name the encoding gave. The line runs here: a Type 1 or CFF encoding is a 256-slot vector of
   names, and a TrueType `cmap` is not.

## Consequences

A subset Type 1 or CFF program showing a code that neither the PDF encoding nor the program names now paints the
designer's `.notdef`, often a box, where it painted nothing. ADR 0932 found no corpus document in that set. A code
meaning a space stays blank. Under a named encoding, a code that encoding leaves empty no longer borrows the
program's own glyph.
