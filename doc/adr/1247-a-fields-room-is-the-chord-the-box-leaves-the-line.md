# 1247 — A field's room is the chord the box leaves the line it is laid on

Status: **accepted**.
Context: `crates/pdf-model/src/variable_text.rs` (`Frame`, `Room`, `Stack`, `Owed`, `lay_out`,
`auto_size`, `wrap`, `write_lines`, `comb`, `overflows`), `crates/pdf-model/tests/variable_text.rs`,
`crates/pdf-model/examples/variable_text_census.rs`.
Builds: ADR 1114 (the box carried back through a diagonal pair), ADR 1130 (the same over a 2×2, by
the line's direction), ADR 0240, ADR 0106, ADR 0348.
Amends: ADR 1130's population — its "one case instead of four" is now none of four, and the one
thing `Owed` still reports about a `Tm` is a different thing.
Clauses: ISO 32000-2 §12.7.4.3, §12.7.5.3 (Table 231), §12.7.5.4, §8.10.2, §9.4.4.

## The refusal's sentence was about a length, and the clause is not about a length

ADR 1130 left one case reported: a linear part with `a` and `b` both non-zero "sends the line off
*both* of the box's axes … so no length the box states is the room that line has". That is true and
it is not a reason. What the clause asks of a processor is

> If this operator is present, the interactive PDF processor shall replace the horizontal and
> vertical translation components with positioning values it determines to be appropriate, based
> on the field value, the quadding ( Q ) attribute, and any layout rules it employs.

— and the only one of those three inputs that needs a *room* is the quadding, which is stated
against the edges of the box the appearance is clipped to (§12.7.4.3 sets `BBox` from "the
dimensions of the annotation rectangle"; §8.10.2 makes that box the clip). So the question is not
whether the box states a length. It is where the box lets a line laid along the matrix's x-axis
image begin and end.

That has an answer for every invertible linear part, and it is closed form. The preimage of the box
is a parallelogram; a line of text is a horizontal segment of the space the layout measures in; and
the two intersect in a chord. The chord *is* the room, and it is the room whether or not it happens
to be a side of the box.

## Two strips, one intersection

The box is two coordinate ranges. Carry each back and it becomes a strip of the measuring space:

    x₀ ≤ a·x + c·y ≤ x₁          y₀ ≤ b·x + d·y ≤ y₁

For a baseline `y` each strip bounds `x` affinely where its `x`-coefficient is non-zero, and bounds
the baseline instead where it is zero. `Room::at` intersects whichever of the two bound the line;
`Room::of` takes the stacking extent from the four corners of the box carried back, which is where
a linear map puts a rectangle's extreme points. The determinant cannot be zero without `a` and `b`
both being zero, so at least one strip always bounds the line and the intersection is never the
whole axis.

**This is ADR 1130's two families restricted, not a construction beside them.** Where `b` is zero
the second strip degenerates to a baseline range and the first gives an interval of fixed length
`|x₁ − x₀| / |a|` sliding by `−c/a` per unit — which is exactly `Frame::shrink` plus `Frame::drift`,
term for term. Where `a` is zero the two exchange roles, which is exactly what `Frame::turned`
recorded. So `Frame::shrink`, `Frame::drift`, `turned`, `along`, `across` and `shear` are all gone,
replaced by the general statement they were a special case of; `Frame::place` is now the linear map
itself and `Frame::shrink_point` its inverse, and the two reduce to what they were in both families.
Nothing a `/DA` could already state moves by a pixel, which is why no fixture of ADRs 1114 or 1130
changed.

## What a varying chord costs the layout, and where it is paid

A chord that is a different length on every line is the whole difference, so every place the layout
used one width now asks which line it is about. `Stack` is that: the room, the metrics, the `/DA`'s
`TL` and the shape, with `first`, `baseline` and `at`. Wrapping asks the room of the line it is
building, which is possible because a block of wrapped lines starts at the top of the box whatever
the count, so line *n*'s baseline is known before line *n* exists. Auto-sizing holds each line to
its own room rather than the widest line to one width. `/Q` quads within the chord of the line it
is placing. A comb divides the chord at its single baseline, single because Table 231 bit 25 "[m]ay
be set only if … the Multiline, Password, and FileSelect flags are clear".

`Set` is gone: its two fields were a size, which is a parameter, and the metrics, which `Stack`
carries.

## What this leaves reported, and why it is a report

`Owed::SingularTextMatrix`, and only that. A linear part with no inverse sends the whole plane onto
one line: the box has no preimage that is a region, no pair of translation components is more
appropriate than another, and the glyph outlines the matrix is written in front of are flattened
onto that line and enclose no area. That is not a layout this tree declined to build — there is
nothing for a layout to decide. The old sentence said such a matrix leaves "no box at all" and then
reported the text as "positioned as if it were the identity", which was the right behaviour under a
description of a different failure.

It stays a **report rather than a refusal** because the marks are still the producer's matrix
applied to the producer's value, which is what §12.7.4.3 leaves standing; what the report adds is
that the positioning half of the clause had nothing to work on.

## What it costs the corpus, and what the fixtures had to show instead

Zero. `examples/variable_text_census` counts three `/DA`s stating a `Tm` at all in each population
and all six are the identity, so no page on this disk moves (trap 8: the corpus cannot rank any of
this). The fixtures carry it instead, and two of them are new:

- **A single line, `/Q 2`, turned half a right angle.** The upright twin's ink ends at the box's
  right edge; the turned one's ends well short of it and at the box's **top** edge, because that is
  where a line at 45° leaves this box. Nothing is owed by either.
- **A block of wrapped lines, upright and turned.** The same value, size and box. A layout giving
  each line its own room draws the whole value both times and the ink counts differ only by what a
  rasteriser makes of diagonal stems; one measuring every line against the box's width hands each
  far more room than the box leaves at its baseline and loses most of the glyphs to the clip.

The honest consequence, recorded rather than smoothed: under a general turn the first line of a
multiline block sits at the parallelogram's topmost point, where the chord is shortest, so such a
block opens narrow and widens as it runs down. That is what the box and the matrix jointly state —
a line laid any wider there would be cut by the clip — and the alternative is a placement rule this
program would be inventing.

## The one witness this clause still declines, and what actually declines it

`freetext_no_appearance.pdf`'s paragraph of Arabic, and the question worth settling is *which*
of three things refuses it: shaping, bidirectional ordering, or the face.

It is the **face**, and the measurement is already in the tree (ADR 0348, `doc/stack.md`): no
compiled-in face has one Arabic glyph — Liberation Sans's `cmap` maps the whole Arabic range to
glyph 0 and its `GSUB` states no `arab` script — and the `/Differences` route is shut
machine-independently, the value having 36 distinct missing characters against the invented array's
31 free codes with no Adobe Glyph List name for any of them. So `encode` produces **no codes at
all** for that value.

UAX #9's ordering is implementable and is not what is missing: run ordering reorders codes, and
there are none. Building it now would be building a mechanism with no input, provable by no fixture
this binary can draw, which is the shape of feature ADR 0133 exists against. Shaping is a separate
decision `doc/stack.md` already records, and it is downstream of the same blocker. The order of
dependence is face, then shaping, then ordering, and nothing above the first can be tested until
the first exists — so ADR 0348's list stays whole, and this round's answer to "what exactly
refuses" is a name rather than a build.
