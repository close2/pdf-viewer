# 2869 — The authoring line's "redaction's overlay" is `/OverlayText` and `/IC`, not `/RO`

Session 2026. Status: **accepted**, on the owner's answer `doc/questions/A842` (2026-11-13, "agreed"), which adopts
Q842's recommendation; it amends the reading `doc/questions/A64` gave the clause-driven relocation amendment.
Clauses: ISO 32000-2 §12.5.6.23 (Table 195).

## The reading

`CLAUDE.md`'s clause-driven relocation amendment takes in a producer's own marks where a clause requires the move
and fixes the position, and stops where the marks become this program's invention. Table 195 gives three overlays,
and they fall on different sides of that test:

- `/RO` is "a form XObject specifying the overlay appearance" — the producer's stream — and the table says when it is
  drawn (after the removal) and where (its origin on the rectangle's lower-left corner). Both halves of the test hold.
- `/OverlayText` is a string this program would lay out under `/DA`, `/Q` and `/Repeat`, and `/IC` a fill this
  program would paint: marks it composes, so they stay excluded, and not drawing them stays a reported departure.

So the exclusion's "the redaction's overlay and its fill" now reads `/OverlayText` and `/IC`; `CLAUDE.md`'s sentence
and `doc/scope-ratifications.md` say so, with A834's and A842's words. The owner's condition is the order: nothing
is drawn over content that is not gone (A834, item 3), and ADR 2868 keeps it by construction.

## Reopened if

A `/RO` is ever asked to be drawn where the removal did not run first, or `/OverlayText` or `/IC` is asked to be
composed — each is a new amendment, argued on its own.
