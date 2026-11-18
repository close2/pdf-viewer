# 3046 — A font stating no descriptor advances a code it gives no width by the glyph painted

Session 2087. Status: **accepted and built.** In `pdf-font`: `metrics::painted_glyph_widths`, read by
`simple_widths`, and `SuppliedWidths::from_drawn_face`; in `pdf-transform`: `described::restated_entries` states
those advances for a converted font. Context: ADRs 2942 and 2948 (the same reading for one of the fourteen stated by
name alone), 2898 and 2906 (the owner's order outside the fourteen, `doc/questions/A859`, `A868`), 2687 (a
descriptor's `/MissingWidth`). Clauses: ISO 32000-2 §9.2.4, §9.6.2.1 (Table 109), §9.6.5.2, §9.8.1 (Table 120).

## Context

Records 2062 and 2064 left a code no encoding names at advance 0. ADR 2948 had since given it `.notdef`'s advance
for a name-alone standard font, so the premise held only outside the fourteen: a font drawn from a substitute that
states no `/FontDescriptor` laid out an empty slot, a name its face lacks, and any code outside a `/Widths` range at
0, and a glyph the substitute holds but the published metrics do not name (a `beta` under `/Arial`) was painted at
0 too, so the next glyph stood on it.

## Decision

1. **Where no descriptor is stated, no `/MissingWidth` is stated.** Table 109's `/Widths` row sends a code outside
   the range to "the value of MissingWidth from the FontDescriptor entry for this font", and ADR 2898's third step
   is that entry; Table 120 is the descriptor's table. A dictionary with no descriptor states neither, and Table
   120's default of 0 is the default of an entry the font never wrote, not a statement. The same holds of one of
   the fourteen that writes `/Widths` without the descriptor Table 109 requires beside them ("shall either all be
   present or all be absent"): a code outside its range was laid out at 0, and is not now.
2. **What is left is §9.2.4's width of the glyph painted**: "the Tj operator shall position the consecutive glyphs
   of a string according to their widths", a width being the distance the text position moves "when the glyph is
   painted". §9.6.5.2 decides the glyph: the name's own in the drawn face, or ".notdef" for a name it lacks; an
   empty slot names `.notdef` (ADR 2948). So such a code advances by the drawn face's glyph, or its `.notdef`.
3. **The owner's order stands where it reads something**: the embedded program's advance, then the resembling
   standard font's published metrics, then a stated descriptor's `/MissingWidth`, stated or 0 (A859, A868). This
   decision is the last step only for a font whose dictionary has no descriptor to read it from, and it is
   reported with the order (`SuppliedWidths::detail` names how many codes the drawn face answered).
4. **The archive states what the window drew.** A converted font is given a descriptor, so an advance its `/Widths`
   do not carry would read the new descriptor's `/MissingWidth`: `described::restated_entries` now writes the
   window's advance for every code, the file's own widths unchanged inside their range (ADR 3047), where it used to
   write them only for a font that stated none (ADR 2146).

## Consequences

`/Arial` with no descriptor sets a `beta` at the drawn face's own width and an empty slot at `.notdef`'s, where
both were 0. A page reaches this only through a font missing an entry Table 109 requires, and the report for a
missing `/Widths` already names it. Tests: `metrics.rs`'s
`a_font_with_no_descriptor_advances_a_code_no_encoding_names_by_its_notdef`,
`a_code_outside_the_widths_of_a_font_with_no_descriptor_takes_the_notdef_advance`, and
`a_substitute_with_no_widths_keeps_the_resembling_standard_metrics` with its no-descriptor case moved from 0.
