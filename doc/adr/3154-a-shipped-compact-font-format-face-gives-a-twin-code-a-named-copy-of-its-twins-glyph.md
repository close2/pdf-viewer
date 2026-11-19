# 3154 — A shipped Compact Font Format face gives a twin code a named copy of its twin's glyph

Session 2113, an archive round in the `lane4` worktree, on `doc/pdfa-blocking-issues.md`'s R10
entry for `499039`. Status: accepted and built. In `pdf-font`: `cff.rs`
(`GivenOutline::GlyphsOwn`, written by `with_glyphs_given`), `type1_glyphs_given.rs` (refuses it by
name). In `pdf-transform`: `archive/fonts/twin_codes.rs` (`split_compact`), `archive/fonts.rs`
(the twin arm for a shipped bare CFF face, `split_twins` by format, the twin encoding in the
restatement's entries). Test: `crates/pdf-transform/tests/archive/twin_codes.rs`
`a_soft_hyphen_in_a_compact_font_format_face_reaches_a_copy_of_its_glyph`. Context: ADR 2723,
ADR 3153, ADR 2578 (`with_glyphs_given`), ADR 2147 (a `TrueType` dictionary given a CFF face is
restated as `Type1`).

## The case

`499039`'s unembedded `TimesNewRomanPSMT` states 333 for code 45 and 0 for code 173 and is given the
shipped Foxit Serif, a bare CFF program, under a dictionary restated as `Type1`. The face names no
`softhyphen` glyph, so ADR 2723's route — rename the code and read the face again — reached no glyph
and kept the fence.

## The decision

A simple font's code reaches a glyph of a name-keyed program by its charset name (ISO 32000-2
§9.6.5.2), so the copy is the name. The code is held to its twin's glyph under the encoding as the
file states it, and `pdf_font::cff::with_glyphs_given` appends a glyph named `softhyphen` (or
`nonbreakingspace`) whose charstring is the twin's, every byte but the width operand, against the
same Private DICT and subroutines; the restated encoding names it for the code, and the advances
are restated as on every substitution. The restatement's own proof reads the font back with that
encoding and holds every shown code to the outline the window drew. Taken only for a shipped face
(a pack face is held to its own code table, ADR 2934) and only where the dictionary keeps its kind
or is restated as `Type1`; a face already naming the glyph keeps the fence, since two glyphs of one
name leave the lookup choosing.

A Type 1 program's glyphs are keyed by name in its `CharStrings` dictionary, not by an index, so
`GivenOutline::GlyphsOwn` is refused there by a sentence of its own.
