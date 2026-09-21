# Q72 — Where the standard names an effect and states no quantity, is *reporting* the documented choice, or is drawing one?

Source: session 1164, sweeping the twelve `departed` rows the owner's `doc/adr_revisit/` note on
ADR 1119 asked for (ADR 1165). Three of the twelve rest on one argument and nothing else, and it is
the same argument four other rows rest on.

## The fact

`CLAUDE.md` says, twice and in two places, what *done* means where the standard defines nothing.
Principle 5, of a clause that requires an appearance and states none:

> say so plainly, make a deliberate choice, and document it *as a choice*

and

> where the standard defines nothing — an annotation icon's artwork, how a fractional page becomes a
> whole number of pixels — done means a documented choice, not a match with anyone.

This tree's practice is the other one. Where a clause names a thing to draw and states no quantity
for it, the mark is **not drawn** and the omission is reported by name:

| row | what is named and not drawn | the argument |
|---|---|---|
| §12.4.4, §12.4.4.1 | `Blinds`, `Glitter`, `Dissolve`, `Fly` | Table 164 states no line count, no band width, no dissolve pattern, no definition of *changes* (ADR 0230) |
| §12.5.6.19 | `/TP` codes 2 to 5 | Table 192 names the side the caption goes on and no proportion of the rectangle (ADR 0239) |
| §12.5.6.4 (§12.5.6.11, §12.5.6.12 beside it) | the predefined icon appearances, a caret's `/Sy`, a stamp's `/IT` | the clause requires the appearances and states not one line of their artwork (ADR 0030) |
| §12.5.6.10 | a highlight, underline, strikeout or squiggle with no `/AP` | `/QuadPoints` and no mark: no thickness, no offset, no period (ADR 0030) |

The refusals are deliberate, argued and reported, so they are not an oversight. What they are not is
what the two sentences above describe: a choice made and written down. And the difference is on the
page. `examples/presentation_census` counts `Dissolve` on 221 pages of 11 crawled documents and
`Blinds` on 16 of 4; a reader stepping through those sees a cut where the producer asked for an
effect, with a sentence in a report a slideshow's audience never reads.

## What is asked

Which of the two readings governs a clause that names a mark and states no quantity for it:

- **Report** — the present practice. A mark this project invented is not the producer's, and
  `CLAUDE.md`'s "as its producer specified" is better served by an absence plus a sentence than by a
  picture nobody specified. Then the two sentences above are about *artwork the standard names as
  existing* (an icon) rather than about every silence, and the rows stay as they are.
- **Choose** — draw it, at a number written down as this project's, tested as this project's, and
  named in the report as chosen rather than read. Then §12.4.4's four transitions, §12.5.6.19's four
  codes, the icon set and §12.5.6.10's four markup subtypes are **owed**, and four `departed`/not-owed
  rows become `partial`.

Nothing in this round depends on the answer; every one of those rows is recorded honestly under
either reading, and ADR 1165 re-derived their premises and left them standing on the argument they
have. What the answer decides is whether that argument is the project's or a habit the project has.

Recommendation: **choose, and bound it**. The two sentences in `CLAUDE.md` are not hedged, and a
viewer that shows a cut for a `Dissolve` has not drawn the page its producer specified. The bound is
the one the refusals already know how to state: a choice is permitted where the clause names the
*kind* of mark and withholds only a **quantity** (how many lines, how wide a band, how much of a
rectangle), and forbidden where the clause names no mark at all — which keeps §12.5.6.23's overlay
and the watermark exactly where `doc/questions/A64` and `A65` put them, on the far side of the
authoring line. Each such choice ships with its number in the code, its ADR, and a report saying the
quantity is ours.
