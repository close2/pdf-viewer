# 3642 — A side bearing `hmtx` omits is taken from the glyph's own box, and the restated program is proved

Session 2235. Status: **accepted and built**; extends ADR 3621 (`horizontal_metric` answers `None`
rather than zero) and ADR 3569 (a program written as this reader read it, proved glyph for glyph).
Context: pdf.js `bug911034`, whose TrueType program's `hmtx` holds all 8 320 long metrics its
`hhea` counts and none of the 42 057 side bearings its `maxp` makes follow them, and whose
`/Widths` disagree with the program, so `fonts/widths-agree-with-the-program` restates the
advances. Clauses: ISO 19005-2 section 6.2.11.5, ISO 19005-4 section 6.2.10.5; ISO 32000-2 §9.6.2
(`/Widths`); ISO/IEC 14496-22 `hmtx` and `glyf`, read through OpenType's own text as the font
format's definition, never as a source of truth for PDF.

## Context

After ADR 3621 a table shorter than its counts declined the whole restating, and the converter
refused `bug911034` with `PROGRAM_NOT_RESTATABLE`, a sentence about Type 1 and CFF charstrings
that says nothing of an sfnt. The advances are all there; only the bearings are missing.

## Decision

1. **`sfnt::horizontal_metric` answers `None` only for a missing advance**; the bearing is its own
   `Option`. Where the bearing is missing, the writers holding the glyph's outline take it from
   the glyph's box, `sfnt::bearing_from_box`: the `xMin` of its `glyf` header, and zero for a
   glyph with no outline. OpenType's `hmtx` says the bearings "may be the same as the xMin values
   in the 'glyf' table, though this is not true for all fonts", and of a glyph with no contours
   that its bearing "should be zero"; where the table states none, the box is the only statement
   the program makes of one.
2. **The restated program is proved** (`as_read::sfnt_draws_alike`, ADR 3569's outline half):
   every glyph drawn unhinted through `skrifa` in the program's units, point for point and bit for
   bit, before and after. A program it does not hold for is declined by name. The advances are
   then checked by `read_back` as before.
3. **Who takes the box.** `sfnt::with_advances` (the metrics route) and `stretch` (whose own proof
   already compares every glyph) take it; `stretch_cff` has no `glyf` and `embed`'s subset is not
   proved, so both still decline a missing bearing.
4. **The converter's sentence for an sfnt is its own** (`SFNT_NOT_RESTATABLE`): `hhea`, `maxp` or
   `hmtx` stating no advance for a glyph, a missing bearing with no box, a failed proof, or an
   OpenType program's CFF width in a subroutine. `PROGRAM_NOT_RESTATABLE` keeps Type 1 and bare CFF.

## Consequences

`bug911034` converts under `as-shown` at 2b (`font_as_read`); the census names the rest.
