# 2942 — A name a standard font stated by name alone does not hold advances by its `.notdef`

Session 2064. Status: **accepted and built.** In `pdf-font`: `metrics::notdef_advanced`, read by `simple_widths`,
and `LoadedFont::advanced_by_notdef`; in `pdf-transform`: `described::from_pack`'s `own_advances`. Context: ADRs
2687 (an embedded standard font with no widths), 2898 and 2906 (the order outside the fourteen), 2146 and 2938 (the
widths a converted standard font states), 0254 (Errata Collection 3 on §9.6.2.1 and §9.6.2.2). Clauses: ISO 32000-2
§9.2.4, §9.6.2.1 (Table 109), §9.6.2.2, §9.6.5.2, §9.8.1 (Table 120).

## Context

A `Times-Roman` stated by name alone — no `/Widths`, no descriptor, as Table 109 lets one of the fourteen — whose
`/Differences` names `beta` was laid out with that code at zero advance. The published metrics name no `beta`
(Annex D.2's Latin set), the shipped Foxit face holds none, so the code reached no glyph and fell to Table 120's
default of 0. The window drew nothing there and set the next glyph on top of where the beta was meant to be; the
archive's pack route (ADR 2938 section 4) drew the pack face's real beta at that zero advance, so the converted file's
glyphs overlapped. Round 2062 found it and left it.

## Decision

**The text fixes the width, and it is the painted glyph's.** §9.6.5.2: "If an encoding maps to a character name that
does not exist in the Type 1 font program, the .notdef glyph shall be substituted." §9.2.4: "the Tj operator shall
position the consecutive glyphs of a string according to their widths", the width being "the distance the current
text position shall move (by translating text space) when the glyph is painted". The one statement Table 109 could
make instead — a code outside `/FirstChar` to `/LastChar` takes "the value of MissingWidth from the FontDescriptor
entry for this font" — needs the range and the descriptor, and a dictionary stated by name alone has neither. Table
120's row says the same where it is stated: it "shall have a predictable effect only if all such codes map to glyphs
whose actual widths are the same as the value of the MissingWidth entry". Zero is therefore not derived; it was the
default of an entry the font never states.

1. **The window.** Inside the fourteen, with no descriptor stated and the font drawn from a substitute, a code whose
   glyph name neither the published metrics nor the drawn face hold is advanced by that face's `.notdef` (glyph 0,
   250 in the Times faces, the same number the code naming `/.notdef` outright already took, `issue4304`). The
   drawing is unchanged: the shipped faces' `.notdef` paints nothing. Nothing is reported; the layout is the
   standard's (ADR 2898 section 3).
2. **A descriptor keeps ADR 2687.** A dictionary that states a descriptor states where `/MissingWidth` lives, so such
   a code keeps it, stated or 0. The paragraph beneath Table 109 that ADR 2687 quoted is struck by Errata Collection
   3 (ADR 0254); this decision does not rest on it.
3. **The archive applies the same rule to the face it draws.** The pack route draws the pack face's own glyph for the
   name (ADR 2934), so the converted file states that glyph's own advance for the code in `/Widths`, and the face is
   not restated there; every other code keeps the window's width. The page after such a code is set by the beta's
   width rather than the shipped face's `.notdef`; that is part of the `machine-face` loss the route already carries,
   which names a face neither the producer's nor the window's.

**Considered and not taken**: stating the window's 250 for the pack's beta, which keeps the window's positions and
overprints the next glyph by the difference; and drawing `.notdef` in the converted file, which would undo ADR
2934's pack for every name outside the standard set.

## Consequences

A line under a name-alone standard font showing a glyph outside its set no longer collapses: the window leaves a
`.notdef`'s gap, the converted file sets the glyph at its own width. Codes no encoding names (an empty slot) and every
font outside the fourteen are untouched.
