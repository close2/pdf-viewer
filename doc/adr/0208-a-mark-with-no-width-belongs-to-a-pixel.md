# ADR 0208 — A mark with no width belongs to a pixel, and a stroke with one does not

Status: accepted, 2026-08-06 (session 368).

## Context

Since the hundred-and-eighty-sixth session a fill whose subpath has no extent along one axis gets
a mark rather than nothing (ADR 0154, `pdf_render::collapsed`). The mark was one device pixel
thick and **centred on the shape's own position**, which is where a stroke of that width down the
same line would sit. An anti-aliasing rasteriser therefore splits it across two pixel rows at
every placement but one, and a page ruled with a grid of such shapes — `issue4260_reduced.pdf`
draws every line of its grid with `848 1085 10159 0 re f` — comes out as a mixture of crisp lines
and fuzzy grey double ones. The project owner saw it beside Okular on a high-DPI screen;
`doc/QUORRA_HAIRLINE_MARKS.md` measured it from the other side and `doc/todo/10` proposed a fix.

Measured with `cargo run --release -p render-quorra --example mark_width`, which renders that page
through both backends at three scales and reports each rule's starting row, rows touched and ink:

| | rows per line | ink per line |
|---|---|---|
| before, both backends, 1×/2×/4× | **1–2**, placement-dependent | 1.00 |
| Okular, on the owner's screen | 1 | 0.13 |

Two renderers, two different halves of the right answer: ours the right amount of ink in the wrong
place, Okular's the right place with an eighth of the ink.

## Decision

### The clause states the answer, and states it three times

§10.7.4's famous sentence is the one this module already quotes — "A shape shall be
scan-converted by painting any pixel whose half-open square region intersects the shape, no
matter how small the intersection is." Reading it for *placement* rather than for existence needs
the two statements beside it, and the round's first finding is that **both were already in
`doc/md/` and neither had been read**:

> NOTE 1 Normally, the intersection of two regions is defined as the intersection of their
> interiors. However, for purposes of scan conversion, a filling region is considered to
> intersect every pixel through which its boundary passes, even if the interior of the filling
> region is empty.

> EXAMPLE A zero-width or zero-height rectangle paints a line 1 pixel wide.

The NOTE is written *for* a region whose interior is empty, which is exactly a collapsed subpath,
and the EXAMPLE names the very shape this corpus document draws. Neither says anything about a
width: both say **pixel**. And the same subclause says which pixel a point is in — "let i =
floor( x ) and j = floor( y ). The pixel that contains this point is the one identified as
( i, j )" — so the mark is the run of whole device pixels the collapsed axis passes through, found
by flooring that axis's device coordinate.

**The strongest argument is the sentence's own stated purpose**, and it is one the proposal in
`doc/todo/10` made and is worth keeping: the rule exists so that no shape disappears "as a result
of unfavourable placement relative to the device pixel grid". A mark whose appearance depends on
where it falls between two pixel centres is placement-dependent by construction. Snapping is what
the sentence is for.

So the proposal was right, and the ADR records that it was *checked* against the clause rather
than adopted: the reading is now anchored on NOTE 1 and the EXAMPLE, which the proposal did not
cite, rather than on the general sentence alone.

### What is snapped, and what is not

In `pdf_render::split_collapsed_fill`, where every backend inherits one answer (trap 2):

- The helper takes the **placement transform** instead of a precomputed width. It derived the
  width from that transform anyway, through `thinnest_line`, and taking the transform is what
  stops a caller from snapping against one space while measuring against another.
- **Only the collapsed axis is snapped.** Along the other one the subpath has a stated extent, and
  what an extent gets on this device is the coverage its area implies — §10.7.4's first documented
  departure, argued in the ledger. Snapping the length as well would push a departure into an axis
  where the clause is being followed.
- **Only under an axis-preserving transform** — `Transform::preserves_axes`, which is a scale and
  a translation or a quarter turn, and which every mark in this corpus rides. Under a rotation or
  a shear the run of pixels a slanted line passes through is a staircase rather than a rectangle,
  and the band of one device pixel remains as the stated fallback. A quarter turn is handled by
  the same arithmetic rather than by a case: the pixel run is found in device space and carried
  back through the transform's own inverse, and an axis-preserving map takes a rectangle to a
  rectangle whether it keeps the axes or exchanges them.
- A transform that states a width while having no inverse — a page flattened onto a line — keeps
  the band. There is no pixel grid left to snap to, and nothing on the device for the answer to be
  wrong about.

### The hairline stroke does **not** move with it

`render-cpu/tests/zero_area_fill.rs` asserted that the mark was byte-identical to a `0 w` hairline
stroke down the same line. That identity is now broken on purpose, and the clauses separate the
two rather than this renderer doing so:

- **A stroke has a width the document stated.** Moving its *coordinates* onto the grid is
  §10.7.5's first requirement — "the line width and the coordinates of a stroke shall
  automatically be adjusted as necessary to produce lines of uniform thickness" — and the same
  clause makes it conditional, on Table 58's `/SA`. `AMBIGUOUS_STROKE_ADJUSTMENT`'s reading of
  `bug1743245.pdf` is a derivation precisely because this tree conditions §10.7.5 on `/SA` rather
  than applying it always; grid-fitting every hairline would turn that into a coincidence.
- **A degenerate fill has no width at all.** Every pixel of its mark is this processor's
  construction under §10.7.4, which states which pixels are covered and attaches no condition —
  and which, in the sentence after the one quoted above, **exempts the zero-width stroke from that
  same rule**: "Zero-width strokes may be done in an implementation-defined manner that may
  include fewer pixels than the rule implies."

That last sentence is what makes this an argument rather than a preference. The standard itself
puts the fill's placement inside the rule and the zero-width stroke's outside it. The test is now
an ink test: it asserts that the two constructions still lay down the *same amount* of ink — one
device pixel per device pixel of length, which is the fact that must not drift, since both come
from `thinnest_line` — and that at a placement off a pixel boundary the two pictures **differ**,
so a reader is told the answers parted on purpose.

## What it moved

Every gate in `doc/todo/02-every-round.md` §2 was run before and after.

| gate | before | after |
|---|---|---|
| `mark_width`, both backends, 1×/2×/4× | 1–2 rows per line, ink 1.00 | **1 row per line, ink 1.00** |
| corpus (974 documents) | 73 incomplete | 73 incomplete, same set |
| oracle (1794 pages) | 897/856 agree, 76/68 contradicted, 786/750 ambiguous, 1/0 our geometry, 2/2 reference geometry, 14/9 not comparable, 18/0 no render | **identical, every bucket** |
| oracle, `issue4260_reduced.pdf` p1 | mean 13.31, worst tile 26.91, differing 10.02%, ssim 0.5619 | mean **13.09**, worst tile 27.75, differing **6.87%**, ssim **0.5835** |
| quorra vs the CPU oracle (957 pages) | 913 agree, 43 differ, 1 refused, 17 not comparable | **914 agree, 42 differ**, 1 refused, 17 not comparable |
| `doc/todo/00` step 7, 743 complete ambiguous pages | −6.404, −1.708, −1.069, then nothing past −0.84 | **unchanged**; the alarm at −1 holds |
| text, dates, XMP, JPEG 2000, conformance | 99.2%, 1514/1545, 318/319, 14 identical, 875 rows | unmoved |

Three things worth reading in that table.

**`issue4260_reduced.pdf` page 1 is the only page in 1794 whose numbers moved at all**, which is
the corpus saying what `doc/todo/10` said: the shape is general, the witness is one document. It
is also why this change could not be ranked by the corpus — it is a *visible* defect with a
population of one.

**The oracle's verdict on that page did not change and was not expected to.** It is `ambiguous`
because the three C references shade a hairline at something under a fifth while this tree and
`hayro` paint it, which is `CONTRADICTED_ANTIALIASED_EDGES`' departure seen from the other side;
the weight, not the placement, is what keeps the page out of `agrees`. What the snap improved is
the shape of the difference — 10.02% of pixels differing to 6.87%, and similarity up — which is
the direction a fix should move a page it cannot settle.

**The quorra gate improved by one page, and the mechanism is worth stating**: `issue4260_reduced.pdf`
left `DIFFERS_AT_THE_EDGES` because two rasterisers have nothing left to distribute differently
once a mark is a whole pixel row. A band at a fractional position is exactly the thing each
rasteriser spreads in its own way; a whole row is not.

## Consequences

- The corpus's page of ruling lines draws the way the clause says, at every placement and every
  scale, on both backends.
- The mark and the hairline stroke are no longer byte-identical, and the test that used to assert
  that identity now asserts the ink they share and the divergence they do not.
- The band is still what a rotated or sheared degenerate fill gets, and no corpus document is
  known to write one. If a document turns up that does, the staircase is the open question, and
  `doc/todo/_scan-conversion.md` is where it belongs.
- `split_collapsed_fill` takes a `Transform` rather than an `f32`. Three call sites — one per
  backend — got shorter, and none of them can now derive a width from a different transform than
  the one the mark is placed against.
