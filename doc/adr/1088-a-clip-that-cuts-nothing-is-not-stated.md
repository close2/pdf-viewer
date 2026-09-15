# ADR 1088 — A clip that cuts nothing is not stated

Status: accepted, 2026-09-15. Session 1074. Answers ISO 32000-2 §10.7.4's clipping sentence on the
graphics backend without the graphics library changing anything, by leaving a clip off a mark it
contains. Takes `issue16473.pdf` and `bug1844583.pdf` off `render-raster`'s differing list and puts
`bug1844576.pdf` and `bug1978317.pdf` on it, and the second half of that is the decision a later
round must not undo.

## What the clause says, and what it licenses

§10.7.4:

> For clipping, the clipping region consists of the set of pixels that would be included by a fill
> operation. Subsequent painting operations shall affect a region that is the intersection of the
> set of pixels defined by the clipping region with the set of pixels for the region to be painted.

§8.5.4 says the same of the shape — "[t]he effective shape is the intersection of the object's
intrinsic shape with the clipping path". ADR 0355 read the requirement out of those two: `S ∩ C = S`
where `S ⊆ C`, so a clip that contains a mark may take nothing from it, and it fixed `render-cpu`'s
composition by taking `min` where `tiny-skia` multiplied. `doc/QUORRA_FEEDBACK.md` §24 offered the
same reading to the graphics library, which multiplies the two coverages in its own shader
(`coverage.wgsl`: `cov * extent.x * extent.y`) and still does.

**The licence is the other half of the same sentence and it had not been taken.** An intersection
with a region that contains the mark *is the mark*, so an encoder that does not state such a clip
has computed the intersection exactly — with no composition at all, and so with no arithmetic for a
device to get wrong. `render_raster::scene::Encoder::cuts_nothing` asks that question per mark.

## The construction, and why it is free of the view

A chain each of whose links is an axis-aligned rectangle admits a rectangle: the intersection of the
links'. A mark whose own page-space box lies inside it is drawn unclipped. Both rectangles are
**page**-space, before the target transform, which is what keeps the answer a property of the
document rather than of this magnification: every affine carries a containment to a containment, and
§10.7.4's pixel region ("painting any pixel whose half-open square region intersects the shape") is a
superset of the geometric region it is built from, so a containment that holds geometrically holds
against the pixels the device actually clips with. Nothing here calls `consume_view`, and a
page-space scene (ADR 0702) survives the change.

Three conservatisms, each because the mark can reach past its own geometry or the region is not one
this can read:

- **a chain with a link that is not a rectangle** — a curved or many-sided region has an inner
  rectangle too and this does not look for it; the population is `/BBox` and `re W n`.
- **a fill with a collapsed subpath** — §10.7.4 gives it a mark one device pixel thick, wider than
  the path and a width the view decides (`Path::collapses` is the memoised test).
- **a stroke that made §8.5.3.2's dot, gave up width to §10.7.4's substitution, or whose width the
  device will resolve to a whole pixel** (§8.4.3.2's zero, §10.7.5's adjustment). The clip is
  therefore asked for *after* the splits in `stroke::encode` rather than before them.

A group keeps its clip whatever it contains: §8.5.4 gives a group's result a shape of its own,
"defined as the union of the shapes of its constituent objects", and nothing bounds that union
without walking the elements.

## What it moved, and the arrival that is the decision

`examples/clip_cost` (new) draws a page twice on each backend, as stated and with every clip taken
off, so for a clip that cuts nothing the difference is the composition. At page scale:

| page | cpu, clipped | cpu, unclipped | raster, clipped | raster, unclipped |
|---|---|---|---|---|
| `bug1844576.pdf` | 849.31 | 933.38 | 933.09 | 933.09 |
| `bug1978317.pdf` | 13 387.41 | 15 165.68 | 15 118.43 | 15 118.43 |
| `issue16473.pdf` | 498.45 | 501.93 | 501.87 | 501.87 |
| `bug1844583.pdf` | 486.46 | 501.76 | 500.93 | 500.93 |
| `issue19083.pdf` | 448.40 | 448.40 | 396.96 | 447.87 |

A clip now costs the graphics backend nothing on the first four. Two of them left the differing
list; the other two **arrived**, and they arrived because the backend short of ink at page scale is
now the oracle — what `render-cpu` composes by `min` there is a mark ADR 0268 already widened to a
device pixel with the given-up width in its alpha, and a widened mark reaches past a box the
document's own geometry sits inside. At 8× the same clips cost the oracle nothing, which is that
substitution's signature. §10.7.4's third sentence ranks the two answers — "The area covered by
painted pixels shall always be at least as large as the area of the original shape" — and it is the
oracle that is under it.

**So the arrival is the device moving onto the geometry, and re-stating the clip on this side to
make the list shorter would be moving the device away from it.** That is the thing a later round
must not do; the page to fix is `render-cpu`'s.

`issue19083.pdf` is what is left of §24 and it is declined by name: its border stroke's outer edge
sits 0.0002 outside the `/BBox`, so the clip genuinely cuts, and the product then costs the page
11.4% of its ink at page scale. `doc/QUORRA_FEEDBACK.md` §24c carries the measurement upstream.

## The gate

`crates/render-raster/tests/clip_that_cuts_nothing.rs` holds the clause on both backends at two
scales against the mark's own area, computed from the geometry and not from either backend, and its
second test holds the other side — a clip that *does* cut still cuts — so a containment test that
answered `true` for everything fails it. Calibrated by planting the rule back off (trap 13): the
contained mark then reads raster **3.0353** at 1× and **3.2853** at 2× against an area of 4, where
with the rule on both backends read 3.9961 and 4.0069.

**The calibration found a defect in the test rather than in the tree, which is the whole of trap
13.** The first draft's mark was a rectangle, and it passed with the rule off: an axis-aligned
rectangular fill takes the device's own rectangle path, where a clip rectangle is met by a
geometric intersection and never by a product. The mark is a triangle now, because it is every
other shape — a glyph, a curve, a stroke's outline — whose coverage comes off the atlas and is
multiplied by the clip's.
