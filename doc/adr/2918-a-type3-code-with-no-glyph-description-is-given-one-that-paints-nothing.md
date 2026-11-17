# 2918 — A Type 3 code with no glyph description is reported, and given one that paints nothing

Session 2051, a PDF/A round in the audit worktree, on what round 2045 found: veraPDF 1.30.3 failed a
Type 3 font showing a code it describes no glyph for, and this tree's validator passed it. Status:
accepted and built. In `pdf-archive`: `table/fonts.rs` (`type3_glyphs_shown_are_described`, under
`fonts/embedded-programs-define-every-glyph-shown`). In `pdf-transform`: `archive/type3_glyphs.rs`
(`describe`), `Rewrite::Type3GlyphDescribed` (`archive/rewrite.rs`, `rewrite/values.rs`), the row's
first answer in `archive/decision/fonts.rs`, its preparation and obstacle in `archive/prepare.rs`.
Tests: the validator's `a_type3_code_its_font_describes_no_glyph_for_is_a_glyph_the_font_does_not_define`;
`crates/pdf-transform/tests/archive/type3_glyphs.rs` (two).
Context: ADR 1942 (the Type 3 `.notdef` rename, the shape taken), ADR 2500 (the empty Type 3
glyph), ADR 2543 (the row's other answer), ADR 2907. Clauses: ISO 19005-2 section 6.2.11.4.1,
ISO 19005-4 section 6.2.10.4.1; ISO 32000-2 §9.6.4 (Table 110, step b), §9.6.5.3, §9.2.4.

## Context

Both parts' embedding subclause ends with the sentence that embedded fonts define every glyph a
file references for rendering (part 2 numbers it 6.2.11.4.1, part 4 6.2.10.4.1). The validator
asked it of fonts with a program and passed over Type 3 fonts, whose descriptions are the
`/CharProcs` streams. ISO 32000-2 §9.6.4 takes a code to its description by the encoding's name
and then the `/CharProcs` key; §9.6.5.3 makes the encoding the whole mapping and Table 110 the
`/Differences` array "the complete character encoding", and step b) says that where the name is not
a key "no glyph shall be painted". A shown code fails at either step.

## Decision

1. **The validator reports both failures** under the existing row, one finding per code: a code
   the encoding names no glyph for, and a name `/CharProcs` holds no stream for (§7.3.10 reads a
   reference to a missing object as `null`). A font whose `/CharProcs` is not a dictionary is
   passed over; a code named `.notdef` is asked like any other.
2. **The converter gives each such code a procedure `w 0 d0`**, `w` the font's `/Widths` entry,
   one stream per distinct width. Nothing is painted, which is what step b) already paints, and the
   width is the one §9.2.4 lays the page out by, so no mark moves and section 6.2.10.5 is met by the
   same edit. It removes nothing, so it is `Mechanical` and every profile carries it.
3. **Names.** A named code keeps its name and the name gains the procedure. An unnamed code is
   given `empty`, `empty.1`, … — one per width, the first neither dictionary uses — appended to
   `/Differences`. The standard names nothing here, so this is a choice: a name the Adobe Glyph
   List gives no character, as the code had none, and the name ADR 2500's empty font uses.
4. **Refused by name**: a font written directly in a resource dictionary, a font whose encoding,
   `/CharProcs`, `/FirstChar` or `/Widths` cannot be read, a code with no width, a font whose shown
   strings passed the survey's budget, a name two codes share at two widths, and a font the
   `.notdef` rename or the width restatement also writes. A font with a program keeps the row's
   other answer (ADR 2543) through ADR 0816's fence.
