# 1130 — The line's direction decides what a box can be measured in

## Status

Accepted, 2026-09-16. Session 1150. Widens ADR 1114's rule from a diagonal pair to a general 2×2,
and narrows what ADR 0211's layout reports under §12.7.4.3.

## Context

ISO 32000-2 §12.7.4.3 admits one `Tm` in a `/DA` and replaces its translation only:

> The default appearance string shall contain at most one Tm (text matrix) operator. If this
> operator is present, the interactive PDF processor shall replace the horizontal and vertical
> translation components with positioning values it determines to be appropriate, based on the
> field value, the quadding ( Q ) attribute, and any layout rules it employs.

ADR 1114 carried out the case where the four remaining numbers are a pair of positive lengths, by
dividing the box by them before measuring anything. It refused three others on the reading that
"[a] rotation puts a line off the single axis this module lays text along; a skew shears the glyphs
relative to the box the layout measures; a negative element mirrors", and a comb on the reading
that Table 231 bit 25 "writes one `Tm` per cell, so the `/DA`'s matrix is dropped there".

Two of those four sentences were about the *arithmetic being harder*, not about the clause. The
third and fourth were wrong.

## Decision

**What decides the question is where the linear part sends the line's direction, and there are
exactly two answers a box can be measured under.**

The layout runs text along text space's x-axis and stacks its lines along the y; a box is a pair of
lengths on the appearance's axes. So:

- **`b` is zero**: the line runs along the appearance's x-axis. The preimage of the box is a band of
  fixed height whose horizontal extent has fixed *length* `|x₁ − x₀| / |a|` and an origin that
  slides with the baseline by `−c/a` per unit. That slide is `Frame::drift` and it is the whole of
  what a shear does to a layout — the advances are untouched, and the glyphs lean.
- **`a` is zero**: the line runs along the appearance's y-axis — a quarter turn, with whatever
  scale, mirror and shear it carries. The same arithmetic with the two axes exchanged.

So a **scale**, a **mirror**, a **half turn**, a **quarter turn** and a **shear** are all carried
out, by the one construction ADR 1114 wrote for the diagonal case. `Frame` is that construction
over a 2×2: `shrink` carries the box back, `place` carries each position forward, and
`Frame::UPRIGHT` is what a `/DA` stating no `Tm` runs under, where every step is the arithmetic
that was there before.

**A comb is not an exception and never was.** Table 231 bit 25 decides *where a character sits* —
"as many equally spaced positions, or combs, as the value of `MaxLen`" — and §12.7.4.3 decides
*what space it sits in*. Both are true at once: the cells are divided out of the box in the space
the matrix maps from, and each cell's `Tm` carries the `/DA`'s linear part. Dropping the matrix
there was reading bit 25 as though it displaced the clause that sends a field to this module.

**What keeps the report is geometric rather than cautious.** A linear part with `a` and `b` both
non-zero sends the line off *both* of the box's axes — a turn by something that is not a multiple
of 90° — and then no length the box states is the room that line has, so no "positioning values it
determines to be appropriate" follow from the box at all. A singular linear part leaves no box.
Those two are `Owed::TransformedTextMatrix`, and it means one thing now instead of four.

## Alternatives

**Lay a line out along a direction.** What ADR 1114 said a rotation would need, and it is still
true of the off-axis case: the room a line has would be a function of where it starts, and every
question in `Asked` — a caret, a point, a selection — would become a shape rather than a rectangle.
Rejected as a different piece of work, which is why the row still says `partial`.

**Take the preimage's bounding box for the off-axis case.** Rejected outright: the text would be
laid out against a rectangle larger than the box, drawn outside it, and clipped — a plausible wrong
page, which is the thing trap 5 exists against.

**Keep answering a selection with a rectangle.** Rejected: under a shear the image of a line's box
is a parallelogram, and a bounding box would overstate what a host highlights. `LaidOut::selection`
answers four corners, which is what `appearance::selection` already turned it into.

## Consequences

`Scale` becomes `Frame` and `Marks::selection` becomes four corners; `appearance::selection` gets
shorter. No public type changes and nothing crosses the ABI. §12.7.4.3's ledger row stays `partial`
and the edge it names is one case instead of four.

**No page on this disk moves.** `examples/variable_text_census` classifies every object the clause
lays text out for by its `/DA`'s linear part — widgets and §12.5.6.6 free text alike — and over
1452 curated and 65 720 crawled documents exactly three in each population state a `Tm` at all, all
six of them the identity. Calibrated on eight planted files, one per class, with two controls.
