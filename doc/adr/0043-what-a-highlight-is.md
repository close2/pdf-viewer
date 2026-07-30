# ADR 0043 — What a highlight is

Status: accepted, 2026-07-30.

## Context

§12.5.6.10's four text markup annotations — Highlight, Underline, StrikeOut, Squiggly — were
the largest thing the corpus still named: 8 documents, 13 annotations, refused with

> §12.5.6.10 states its `/QuadPoints` without stating what mark to make in them

and ADR 0030 argued the refusal at length: "the standard states no mark", "the three references
draw three different pictures", "any implementation here is a documented choice, and it should
be argued as one".

The spec item beside it was `REVIEW_OWED`, which had two entries left — §7.9.5 and §14.11.5 —
and both are one row of a family nobody had read.

## The refusal was reading the clause for the wrong thing

§12.5.6.10 states more than the entry list suggests:

- **The mark.** These annotations "shall appear as highlights, underlines, strikeouts (all PDF
  1.3), or jagged ("squiggly") underlines in the text of a document". Four subtypes, four named
  marks.
- **The region.** Table 182's required `/QuadPoints`, "an array of 8×n numbers specifying the
  coordinates of n quadrilaterals in default user space", each of which "shall encompass a word
  or group of contiguous words".
- **The orientation.** "The text shall be oriented with respect to the edge connecting points
  ( x 1 , y 1 ) and ( x 2 , y 2 )."
- **The colour**, from Table 166's `/C`, which every constructed appearance already uses.

What it does not state is a *thickness*, where a strikeout crosses, and a squiggle's period.
That is a much smaller hole than "no mark", and the difference between the two is the whole
finding: **a clause that leaves a dimension is not a clause that states nothing.** The
comparison worth keeping is a `Text` annotation's icon, §12.5.6.4, which names `/Comment`,
`/Key` and `/Note` and states not one line of their artwork. That one is still refused, and now
the two refusals mean different things.

## Decision

### The height is the only length there is, so every dimension is a fraction of it

A quadrilateral "shall encompass a word", so its height is the text's height. A thickness of a
sixteenth of it, a squiggle of a twelfth in amplitude and a third of the height in wavelength,
and a strikeout across the middle are choices *at that scale*. Stated as fractions rather than
as points, they are right at every font size instead of at one.

### Two readings of "counterclockwise", and a construction that needs neither

Table 182 says the four vertices are "in counterclockwise order"; Figure 84 shows (x1, y1) and
(x2, y2) as the **top** edge, which in a y-up space is not counterclockwise. Every producer
follows the figure. Rather than choose, `Quad::read` uses the clause's one unambiguous sentence
— the edge from (x1, y1) to (x2, y2) is the text's direction — and orders the opposite edge's
two vertices by projecting them onto it. That is true under either reading.

Which edge is the *bottom* is then the single open question, and it is answered by the page
rather than by the clause: the lower midpoint in default user space, ties going to the second
edge, which is Figure 84's arrangement.

### A highlight multiplies, and the imaging model is why

The clause says these annotations appear "**in the text** of a document", and §12.5 draws an
annotation *over* the page — so a wash painted normally is not a highlight in the text, it is a
rectangle instead of it. §11.3.5.2 defines exactly one blend mode whose "result colour is always
at least as dark as either of the two constituent colours", which is the standard's own
guarantee that what was under the wash survives it.

So the highlight is its quadrilateral filled under `Multiply`, selected by an `/ExtGState` that
is the constructed stream's only resource. This is the first constructed appearance to need one.

## Consequences

| | before | after |
|---|---|---|
| corpus documents drawing with nothing reported | 848 | **856** |
| pages we claim to draw completely | 1645 | **1653** |
| agreeing with the reference consensus | 802 | **808** |
| contradicted by it | 88 | **88** |
| ledger subclauses nobody has read | 369 | **348** |
| **cited clauses still owing a review** | 2 | **0** |

**Eight pages joined the judged set and six of them agree; none is contradicted.** For a
construction whose dimensions are this project's own choice, that is the evidence that matters:
where the clause leaves a fraction, any sane fraction lands inside the tolerance, and where it
states something — the mark, the region, the orientation, and Multiply through §11.3.5.2 — the
consensus is what agreeing with it looks like.

`REVIEW_OWED` is empty for the first time. It stays as an empty ratchet rather than being
deleted: a clause the code cites and nobody has read is the cheapest debt this project can
accrue, and an empty list fails loudly the moment one appears. Clearing it took two family
reviews — §7.9's common data structures, where three rows are `reported` because a viewer that
navigates will need name trees, number trees and dates and this one does not yet, and §14.11's
prepress support, where six of seven subclauses describe a press and the seventh is the output
intent this tree has read since ADR 0009.
