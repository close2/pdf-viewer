# 3326 — A font's codes are restated over the face the window drew for an unreadable program

Session 2156. Status: accepted. Context: ISO 19005-2 6.2.11.4.1, 6.2.11.6, 6.2.11.8; ISO 32000-2
§9.6.5.4; ADR 2744 (restating over the substitution's face), ADR 2870 and `doc/questions/A837` (a
program present and unreadable is drawn from a substitute).

## The defect

`507676`'s `/R19` (`XIMJKA+TTE201B838t00`) and `/R29` are symbolic TrueType fonts whose `/FontFile2`
decodes whole under an Adler-32 that disagrees, which `pdf-font` refuses (ADR 0836) and the window draws
from `LoadedFont::substitute_for_unreadable`'s face. The substitution embeds that face for them, and
`truetype_cmap::codes_drawn` restates their codes over it — but asked `LoadedFont::load`, which refuses
the program, so the restatement failed for the whole file and three rows (6.2.11.4.1's glyphs, 6.2.11.6,
6.2.11.8) were refused at all twelve fonts. Printed at the refusal: `Malformed { detail: "/FontFile2
decoded whole and its check value disagrees (13419 bytes) …" }` at `codes_drawn`, font 19.

## Decision

`codes_drawn` asks the window's question: `load`, and where it refuses, `substitute_for_unreadable`
over that refusal — what `archive::fonts::drawn_by_the_window` already asks. The proof that follows is
unchanged: each code must reach the shipped face's glyph, with the same outline, in the embedded program.

## Cost

None: a font the window draws from no face still answers `None` and is refused as before.
