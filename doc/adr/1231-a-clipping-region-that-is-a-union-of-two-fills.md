# ADR 1231 — A clipping region that is the union of two fills, composed where it can be and named where it cannot

## Status

Accepted. Closes the fourth departure §10.7.4's ledger row records — "no backend's clip
vocabulary states a union" — on the one backend whose mask it owns, and turns the other two
backends' substitution into a refusal. Amends ADR 1064, which built the substitution; ADRs 1060,
1082, 1088, 1095 and 1102 are the other departures' and are untouched.

## Context

§10.7.4 defines a clipping region by a fill:

> For clipping, the clipping region consists of the set of pixels that would be included by a
> fill operation.

§8.5.4 says the same from the operator's side — "For a given path definition, the same area that
would be filled by the f operator is the area that would be used for a clip" — and §10.7.4's own
EXAMPLE says what a fill of a flat rectangle includes: "A zero-width or zero-height rectangle
paints a line 1 pixel wide".

So a clipping path that encloses an area *and* rules a line admits both. ADR 1064 built the two
halves as two fills for the fill operator, and for the clip it appended the rule's mark to the
path wherever appending it *was* the union — outside the bounding box of the part that encloses
an area — and dropped it where it was not. What no clip vocabulary states is the union itself:
`vello`'s `push_clip_layer`, `raster_scene::SceneBuilder::clip` and `tiny_skia::Mask` each take
one path and one rule.

Session 1185 priced closing it and left it, on the argument that closing it in one backend would
buy the clause and lose the cross-backend agreement the oracle rests on. That argument is the
one this ADR disagrees with, and the disagreement is about what the other two backends do rather
than about what the first one can.

## Decision

**1. The two fills are handed to the backend as two.** `pdf_render::clip_region` returns a
`ClipRegion`: `One(path, rule)` where the union is also one fill under one rule, and
`Union { filled: (path, rule), marks }` where it is not. Nothing about the union is decided in a
backend.

**2. `render-cpu` composes it, and the composition is a sum capped at the pixel.** Its mask is
bytes it owns, so the second fill goes into the scratch mask and is added into the first
(`scan::mask_union`). The sum is chosen over `max` and over the source-over `a + b − ab` a second
`fill_path` would perform, because neither of those is the area of the union at a pixel the two
fills both reach partly, and §10.7.4 states the direction to be wrong in: "The area covered by
painted pixels shall always be at least as large as the area of the original shape". The
clause's own answer for a region is larger still — a filling region "is considered to intersect
every pixel through which its boundary passes" — so the sum lies between this backend's
anti-aliased departure and the clause's set, and it is exact wherever the two fills do not
overlap inside one pixel.

**3. `render-raster` and `render-gpu` refuse a `Union` by name.** Not because the union is
inexpressible in principle — `raster_scene::MaskKind::Alpha` would state it, and a vello layer
composited `DestIn` would — but because in both a mask *multiplies into a draw that already
carries the one the document gave it*, so the two would have to be composed into a third the
scene has no vocabulary for, and on the device it costs a readback a scene under composition
cannot make. The frame goes to the CPU backend, which `CLAUDE.md` keeps for exactly this. That
is what keeps the cross-backend agreement session 1185 was protecting: the two backends do not
draw a different page, they draw none.

**4. The substitute's two over-drops are removed.** The test for whether appending a mark *is*
the union now asks each area-enclosing subpath's own rectangle rather than one rectangle drawn
around all of them at once — a point outside *that* has winding zero and crossing count zero
from *that* subpath, so a mark in the gap between two separated subpaths is appended where it
used to be dropped. And the rectangles are compared for a **positive** overlap rather than by
`Rect::intersection`, which answers `Some` for rectangles that only touch: a mark whose rectangle
merely abuts an area subpath's has its whole interior outside that subpath. What is left after
both is not a substitute at all — it is the union, either composed or named.

## Consequences

The evidence is `render-cpu/tests/zero_area_clip.rs`'s
`a_clip_that_encloses_an_area_and_rules_a_line_admits_the_union_of_both`: a 30-unit square with
a 55-unit rule across it, under `W n` and `W* n`, at scales 1 and 2. Every edge is on a pixel
boundary on purpose — where a boundary falls inside a pixel the fill composites its two parts
source-over while the region sums them, and the comparison would be of this backend's
anti-aliasing departure with itself. Calibrated by planting the old drop back: the clip admits
900 pixels where the fill paints 925, which is the 25 device pixels of whisker exactly.

**The even-odd operator is the discriminator and is why both are asserted.** Appending the mark
to the square's path states the union under `W n` and states a *hole* under `W* n`, where the
crossing counts twice and the parity cancels. A backend that concatenates passes the first line
and fails the second.

`render-raster/tests/clip_region_union_refusal.rs` and `render-gpu`'s
`a_clip_whose_region_is_a_union_of_two_fills_is_refused_and_the_square_alone_is_drawn` hold the
two refusals, each beside the same square without its rule, which both backends draw.

**No corpus page is expected to move, and that is a control rather than a result** (trap 8,
trap 13). §10.7.4's ledger row carries the census ADR 1064 took: `clip_region` is reached over
the corpus's first pages and substitutes a region on none of them, because no first page of the
pdf.js corpus states a clipping path with a subpath collapsed along exactly one axis. So the
fixtures are the whole of the evidence here, as they were for ADR 1064, and `raster_golden`
holding is the control.

**What the refusal costs is a fallback, and it is bounded by that same census.** A document that
reaches it draws on the CPU backend and is reported; the population that could is the population
`clip_region` substitutes for, which no corpus document is in.

`doc/QUORRA_FEEDBACK.md` section 51 is the ask that would let raster state it: a clip whose
region is two outlines under two rules, composed by union before the chain intersects it.
