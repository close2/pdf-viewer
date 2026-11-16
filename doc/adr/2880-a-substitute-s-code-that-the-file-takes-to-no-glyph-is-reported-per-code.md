# 2880 — A substitute's code that the file takes to no glyph is reported per code

Session 2032, a font round in the audit worktree, on round 2027's finding (`issue13316_reduced`).
Status: accepted and built. In `pdf-font`: `substituted.rs`'s `fill_substitute_table` (the `post`
step), `LoadedFont::unreached_in_substitute` and `UnreachedCode`. In `pdf-model`: `Coverage`'s
`unreached` and `first_unreached`, `draw_program_code`'s arm, and the page's report in `content.rs`.
Tests: `crates/pdf-model/tests/substitute_unreached_codes.rs` (all four);
`crates/pdf-font/src/substituted.rs::a_name_the_glyph_list_cannot_map_is_looked_up_in_the_faces_post_table`.
Context: ADR 0007 (a font naming no program is drawn from a substitute), ADR 0152 (a substitute
that draws nothing), ADR 2870 (nor does a program present and unreadable).
Clauses: ISO 32000-2 §9.6.5.1, §9.6.5.2, §9.6.5.4, §9.7.4.2, §9.10.2.

## Context

`issue13316_reduced` stays blank after ADR 2870 while its page says "drawn from a substitute face
instead". Its `/Differences` names codes 65 and 67 to 70 `/g5167`, `/g11927`, `/g17737`, `/g11540`
and `/g2180`, and 66 `/space`. The brief said its `/ToUnicode` does not decode; it does (65 is
U+5F00), and nothing in the clauses makes it select a glyph. The page reported nothing per code
because ADR 0152's report fires only where a font drew no code, and `/space` drew.

## What selects a glyph in a substitute

A substitute is no program the document named, so a code reaches it only through what the code
means. For a simple font that is the glyph name its encoding gives it (§9.6.5.1's table "that maps
from character codes to glyph names"). §9.6.5.4 takes the name through "the Adobe Glyph List and
Adobe Glyph List for New Fonts" to a character for the `cmap`, and then: "if the glyph name cannot
be mapped as specified, the glyph name shall be looked up in the font program's "post" table (if
one is present)". A bare CFF face keys its glyphs by name, as §9.6.5.2 states for Type 1. For a
composite font naming no program, "CIDs shall not participate in glyph selection" (§9.7.4.2), and
the character §9.10.2 gives a code is the route ADR 0007 takes. `/ToUnicode` selects no glyph of a
simple font: §9.10.2 states it for extracting text, and §9.6.5's routes start at the encoding.

## Decision

1. **The `post` step is taken in a substitute face.** The face stands in for the font program, so
   a name the Glyph List cannot map is looked up by name in its `post` table; `.notdef` and glyph
   0 are no mark of the producer's and are not taken.
2. **A code whose route ends at what the file states is reported per code**, whatever else the
   font drew: a simple font's code the encoding names nothing, or names with a name that is no
   character under the Glyph List and no glyph of the face. No face on any machine draws such a
   code, so the statement is about the file and the page says it, naming the count, the first
   code's name or its want of one, and the clause whose route ended. A composite font's code
   reaches its substitute by the character §9.10.2 gives it, so one with none names no mark
   (decision 4), and one the face lacks is decision 3's.
3. **A character the machine's face lacks stays ADR 0152's**, said only where the font drew
   nothing: that is a statement about the machine, and its price in judged pages was measured.
4. **A code counts only where §9.10.2 names a mark it owed** — a character that is not white
   space. A code the encoding names nothing and §9.10.2 names nothing states no mark anyone can
   name, and keeps the account it had: measured, counting such codes reported `issue18059`'s
   two-byte strings through a single-byte `/Helvetica`, `standard_fonts`' control codes and
   `issue20489`'s GBK bytes through WinAnsi, all codes the file names nothing for (trap 11).

## Consequences

`issue13316_reduced` says its five codes, `/g5167` the first, and stays blank: the file states no
route to a CJK glyph through a Latin face's names. Pages with such codes in a font that drew others
become incomplete; the round's record lists the corpus's movement.
