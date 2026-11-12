# Q837 — Should the window draw a substitute for a font whose program it cannot read?

Source: round 2006 (ADR 2830), the PDF/A lane, R09 of `doc/pdfa-blocking-issues.md`.

## What stands

A font whose embedded program is damaged — its `FlateDecode` data stops short, is refused from its
first byte, or fails its checksum, or the bytes are not a program in the format they are filed
under — is drawn by the window from the dictionary's widths alone, marking nothing (ADR 1094). A
font that names no program at all is drawn from a substitute face (ADR 0007). So the same text is
blank when the producer embedded a broken program and readable when the producer embedded none.

The archive now follows the window: ADR 2830 writes such a font with a program whose every shown
glyph draws nothing, so `as-if-printed` and `as-shown` keep the blank, under the new loss
`unreadable-program`. Before, the archive drew the text in a shipped face where the window drew
none. The witnesses: `issue11651` (pdf.js; "Hello world Test" blank), govdocs1 `178360` (a whole
newsletter blank but its frames), `435321` and `268346`.

## The standard

ISO 32000-2 §9.9 counts the front of a program as no program (ADR 2619 reads it so). §9.1 says a
font dictionary carries "information that can be used to provide a substitute when the font program
is not available", and §9.8.1 that the descriptor's metrics "enable a PDF processor to synthesise a
substitute font or select a similar font when the font program is unavailable". Both permit a
substitute; neither requires one. Nothing forbids the blank either.

## The question

Should the window draw a font whose program it cannot read as it draws a font with no program — from
a substitute face, said out loud as the refusal is today — rather than as nothing?

## Recommendation

Yes. The producer's text is fully stated (codes, widths, names, often a `/ToUnicode`), and a reader
who opens `178360` should see the newsletter, not its frames; the standard's own description of the
dictionary's information is exactly this case. It changes ADR 1094's "metrics only means metrics
only" for this one cause (a program present and unreadable), not for §9.7.5.2's forbidden
combinations. The archive would follow in the same change: `cut_program` reads a font the window
loads as a program drawn whole, so the window's substitute must be told apart there and given the
face route ADR 2636 built, without the emptying. Until you answer, the window and the archive both
leave the text blank.
