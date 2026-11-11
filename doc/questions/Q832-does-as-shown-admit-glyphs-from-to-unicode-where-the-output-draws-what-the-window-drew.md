# Q832 — Does `as-shown` admit `glyphs-from-to-unicode` where the output draws what the window drew?

Source: round 1998 (ADR 2807), the PDF/A lane, R09 of `doc/pdfa-blocking-issues.md`.

## What stands

An `Adobe-Identity` composite font the file does not embed has CIDs that index a program nobody
has (ISO 32000-2 §9.7.4.2). The window draws it anyway: each code to the character the font's
`/ToUnicode` states, that character to a glyph of a face (ADR 0270, A570). A570 named two losses for
the archive: `machine-face` (a shipped face, not the one the window drew) and
`glyphs-from-to-unicode` (the glyphs chosen through an extraction aid, which a producer may have
written for searching), and `as-shown` stops on both.

ADR 2807 now embeds the face the window drew, under the profiles that withhold `machine-face`, with
each CID sent to the glyph the window drew for it and proved code by code. For such a font the
output draws exactly what the window showed, and `machine-face` is gone. `glyphs-from-to-unicode`
is still named, because the glyph choice still came from the `/ToUnicode`, and `as-shown` still
stops on it. Four documents of the pdf.js corpus stop there at each part (`issue11651`,
`issue20065`, `issue2840`, `issue9084`).

## The question

`as-shown`'s yardstick is "would a page drawn at rest in the window have lost this?" Where the
output draws, code by code, what the window drew, the answer for `glyphs-from-to-unicode` is no:
the window chose the same glyphs from the same map. Should `as-shown` admit
`glyphs-from-to-unicode` in that case, which is only when ADR 2807's proof held?

## Recommendation

Yes, and only there. The loss would still be named in the report and in `xmpMM:History`, as a loss
that `as-shown` admits. `refuse-any-loss` and the other profiles that withhold it would keep their
stop. Under `as-if-printed` nothing changes. Until you answer, the four documents keep their stop.
