# 3119 — A font outside the fourteen that states no `/FontDescriptor` is drawn as before, and said

Session 2104. Status: **accepted and built.** Context: ADR 2898 and ADR 3046 (its layout), ADR 3047 (the archive's
conversion of it), ADR 2522 (fonts the archive defines inside the fourteen), `LoadedFont::states_no_descriptor`,
`pdf-model`'s `note_missing_descriptor`. Clauses: ISO 32000-2 §9.6.2.1 (Table 109), §9.6.2.2, §9.8.1.

## What it did

A simple font outside §9.6.2.2's fourteen with no `/FontDescriptor` loads as a font the file did not embed: a
substitute chosen from `/BaseFont` and `/Encoding`, laid out by its `/Widths` (a code outside them by the painted
glyph, ADR 3046; no `/Widths` at all by ADR 2898's order, which is reported). With `/Widths`, nothing was said.

## The reading

Table 109's row is "Required; optional in PDF 1.0-1.7 for the standard 14 fonts". ISO 32000-1 had already said, of
the same entries, "Beginning with PDF 1.5, the special treatment given to the standard 14 fonts is deprecated", and
ISO 32000-2 keeps the exemption for 1.0 to 1.7 only, with "PDF processors shall provide glyph widths and font
descriptor data for those standard fonts" when they are absent. Outside the fourteen the entry is required in every
version, so such a file is malformed, and the standard states no repair. §9.8.1 says what the entry is for: "These
font metrics provide information that enables a PDF processor to synthesise a substitute font or select a similar
font when the font program is unavailable."

## Decision

The drawing is unchanged: a substitute by name, the file's widths. **That is a documented choice on a malformed file**,
and the cheapest honest one — refusing the font would lose the text, and every reading the descriptor would have fed
(flags, weight, PANOSE) has a stated default. What changes is that it is said: the page notes "font F states no
/FontDescriptor, which ISO 32000-2 Table 109 requires outside the fourteen standard fonts, so its substitute was
chosen by /BaseFont and /Encoding alone", once per font per page, classed as the file's defect by the corpus walk. It
fires on the table's condition exactly: a simple font, not one of the fourteen, no descriptor. ADR 2522's fonts are
inside the fourteen and are not said; this reader's own stand-in font is not said, as for the widths note.
