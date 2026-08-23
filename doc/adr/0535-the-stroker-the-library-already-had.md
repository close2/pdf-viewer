# ADR 0535 — The stroker the library already had

Status: accepted, 2026-08-23. Session 690. Draws a stroke wider than one device pixel as the fill
of its own outline, so that §10.7.4's clipping paragraph reaches it as a **set** where
`tiny_skia::PixmapMut::stroke_path` multiplied the finished mask into the mark's coverage. Closes
the stroke bullet of `doc/todo/11` item 4; takes a butt-capped axis-aligned rule out of item 7's
remainder as a consequence; amends §10.7.4's ledger row, `doc/todo/_scan-conversion.md`'s
departure (4), `scan::stroke`'s own doc comment and `render-quorra`'s differing list. **It costs
nothing measurable**, and the price the item was carrying did not exist.

## What was owed

ADR 0280 made a clip *chain* compose as a set intersection, ADR 0355 made a **fill**'s own coverage
meet its clip by `min`, ADR 0363 did the same for a clip standing beside a soft mask, and ADR 0492
for an opaque group's raster. What `doc/todo/11` item 4 still listed was a stroke's coverage, an
image's edge and the rest of a group's.

The clause does not distinguish the operators. §10.7.4:

> For clipping, the clipping region consists of the set of pixels that would be included by a fill
> operation. Subsequent painting operations shall affect a region that is the intersection of the
> set of pixels defined by the clipping region with the set of pixels for the region to be painted.

and §8.5.4:

> The effective shape is the intersection of the object's intrinsic shape with the clipping path;
> the source shape value shall be 0.0 outside this intersection.

"The region to be painted" and "the object's intrinsic shape" say nothing about which operator
painted it, and §8.4.3 gives a stroke a shape — the stroked outline. So the closed form ADR 0355
tested a fill against binds a stroke unchanged: `S ∩ C = S` where `S ⊆ C`, so **a clip coincident
with a stroke's outline takes nothing from it**, the anti-aliased boundary included.

`render-cpu` painted it at the square instead. `scan::stroke` hands the mask to
`tiny_skia::PixmapMut::stroke_path`, whose non-hairline branch passes it straight to `fill_path`,
which multiplies.

## The price the item was carrying, and why it was not real

`scan::stroke`'s doc comment and item 4's bullet both said the same thing:

> Rasterising the coverage here would mean choosing between duplicating the library's stroker and
> contradicting its hairline.

**The first half does not exist.** `tiny_skia::Path::stroke` is public and is the same `PathStroker`
`stroke_path` calls; `tiny_skia::Path::dash` is the same dasher; and `stroke_path`'s own non-hairline
branch is *exactly* those two followed by a `fill_path` under the non-zero rule. Reaching the
composition therefore takes no stroker of ours and no scan converter of ours — it takes performing
the library's own two steps one call earlier, so that `scan::fill` composes what comes out.

This is the fourth time in this block that a price fell to the same question, and the question is
`doc/habits.md`'s: **ask what the libraries and the layers already contain.** ADR 0355 found
`Mask::fill_path` and `PixmapMut::fill_rect` where a blitter had been priced; ADR 0476 found a
*second scan converter* where nine calls into the first had been priced; ADR 0492 found the shape
already in the raster where a shape channel had been priced. And the tree already made this exact
move three times over: `draw_long_mitres` has filled a stroker's outline since ADR 0398, and
`draw_rule_as_bands` and `draw_rule_at_one_pixel` have filled substituted outlines since ADR 0226
and ADR 0268. **Three of this backend's four stroke constructions were fills already**; the fourth
— the ordinary one — was the only one still going through the library's own composition.

## The second half: the boundary was the library's

The hairline half is real, and it is answered by moving the boundary rather than by crossing it.

`tiny-skia` decides between a hairline and an outline in `treat_as_hairline`: it maps the width
along the transform's two basis vectors, takes `fast_len` of each — `max + min/2`, an
overestimate of the length by up to 12% — and hairlines when both are at most 1.
`pdf_render::thinnest_line` is the linear part's larger singular value and is exact. The two agree
for every **similarity** transform, which is what a page transform is; under a shear they part by
up to a factor of √2, since `σ₁ ≤ √(|c₁|² + |c₂|²)`.

Trap 2's rule decides it: *where two backends are the oracle, a decision either can make alone is a
decision neither has made* — and a decision a **library** makes for one backend is worse, because
nobody wrote it down. So the boundary is `pdf-render`'s:

- at or under `thinnest_line`, §10.7.4's own substitutions own the mark (`draw_sub_pixel_rule`,
  ADRs 0226, 0268, 0285, 0419, 0420);
- above it, the stroke's own outline does (`draw_stroked_outline`, this ADR);
- `tiny-skia`'s hairline is reached only where `carries_coverage_as_alpha` has already withdrawn
  every substitution — a blend mode of `Source` or anti-aliasing off — which is `scan::intersected`'s
  own first decline too, so nothing is lost there that could have been composed.

## What fell out: a stroke's mark is now a fill's mark

A butt-capped straight rule along a device axis outlines to **one axis-aligned rectangle**. That is
the shape ADR 0476's closed form is about — §10.7.4 defines a pixel as a product of two half-open
intervals and gives a filled shape the same form, so the coverage is the product of the two
overlaps — and it had never reached a stroke, because a stroke was not a fill. `draw_stroked_outline`
asks `pdf_render::device_rectangle` of the outline, behind a bounded verb count so that an ordinary
path pays a comparison rather than a conversion, and hands the answer to the same
`scan::fill_rectangle` an `re f` goes to.

This is item 7's debt paid for item 7's own shape, reached from the other end, and it is *why* the
two operators can now be asserted to be one mark:
`clip_intersection.rs::a_stroke_and_the_fill_of_its_outline_are_one_mark` states `MARK` twice — as
`re f` and as a butt-capped rule as wide as `MARK` is tall — and demands the two rasters agree
within a level. It read **three levels apart** before this round, and it would have gone on reading
that had the composition been taken without the rectangle.

**It also has no dependency on item 5**, which was the first question asked of this work.
`Path::stroke` produces one path, filled once under the non-zero rule, which is what `stroke_path`
does with it anyway — no mark is split into several and no §11.3.7.3 seam is created. Item 7's
*remaining* half is the one with that dependency, and only for a path stating several rectangles.

## What it moves, measured

The instrument is `pdf-model/examples/coincident_edge_probe`, which grew the **operator** as an
axis: the same rectangle painted by `f` and by an `S` whose outline is that rectangle, each
restated four ways and each of those with and without a soft mask worth 1.0 everywhere.

```text
  one rectangle whose lower edge covers 0.504 of device row 9, read at column 20

  the mark is a fill                   the mark is a stroke of the same outline
                 before   after                        before   after
  the mark alone 0.5059  0.5059        the mark alone   0.4980  0.5059
  W n clip       0.5059  0.5059        W n clip         0.2510  0.5059
  form /BBox     0.5059  0.5059        form /BBox       0.2510  0.5059
  group /BBox    0.5059  0.5059        group /BBox      0.2510  0.5059
```

0.5059 is one eight-bit level off the shape's own 0.504; 0.2510 is the product. The "alone" rung
moving is the rectangle half, and the three restatements are the composition half.

**The corpus.** Every oracle verdict count is identical — 902 agree, 60 contradicted, 768 ambiguous,
2 our geometry, 2 reference geometry, 42 not comparable, 18 no render — and **25 of 1794 per-page
lines moved**, 23 of them in the third or fourth decimal place. The two that moved further both
moved *toward* the references:

```text
  issue19083.pdf p1        mean 7.45 -> 7.38   worst tile 16.44 -> 15.32   ssim 0.8333 -> 0.8543
  issue4402_reduced.pdf p1 mean 13.60 -> 12.42 worst tile 16.69 -> 15.45   ssim 0.8889 -> 0.9062
```

**The cross-backend gate**'s differing list went from 22 names to **23** — the run after this change
reports 932 agree, 23 differ, 2 refused, 17 not comparable — and the arrival is `issue19083.pdf`. It is worth naming what that page is, because it is not a new population: its
whole content is `0.5 0.5 124.2502 19 re s` at the default `1 w` inside a `/BBox` of
`[0 0 125.25 20]` — a §12.5.5 widget appearance whose border rule sits on the box §8.10.1 step c)
clips it by, which is exactly the four pages ADR 0355 parted on, with `s` instead of `f`.
`render-quorra` composes a clip *chain* by `min` (its ADR 0030) and still multiplies where the clip
meets the **mark**, on both operators, which it records as a choice — so this is
`doc/QUORRA_FEEDBACK.md` section 24's ask reaching one page further, and section 24b is the
write-up.

**The cost**, `callgrind_rasterise` under `RAYON_NUM_THREADS=1`, five rasterisations per arm:

```text
  ISO 32000-2 p101 (text)            1,489,609,548 -> 1,489,578,526   -0.002%
  ISO 32000-2 p6   (page-wide clip)  1,022,866,656 -> 1,022,782,500   -0.008%
  colors.pdf p1    (16 rectangles)     404,522,657 ->   404,520,696   -0.000%
  22060_A1_01_Plans.pdf p1          22,210,532,588 -> 22,210,343,131  -0.001%
  issue19517.pdf p1                 38,707,670,799 -> 38,707,673,043  +0.000%
  issue12295.pdf p1                 13,411,694,203 -> 13,417,578,181  +0.044%
```

It is a call-for-call substitution of the library's own two steps, so there is nothing to pay for
where the branch fires; `issue12295.pdf`'s +0.044% is the branch's *predicate* alone, since all
65 859 of that page's sub-pixel strokes decline it. The three pages that fell are the rectangle
route replacing a supersampled path fill, which is ADR 0476's measurement again.

## What was planted first

Trap 13: the three new scenes in `render-cpu/tests/clip_intersection.rs` were written and run
against the unfixed tree before a line of `render-cpu/src/lib.rs` moved. They failed by **51 levels
of 255** at the named boundary pixel — `(216, 178, 216)` where the unclipped mark is
`(191, 127, 191)` — by 24 levels under a soft mask, and by 3 levels against the fill of the same
outline.

## What is left of item 4

An image's edge, which is `draw_pixmap`'s; a **non-isolated** group's raster and a group whose
opacity is below 1.0 somewhere, which need the shape channel ADR 0492 priced; and both other
backends, which compose inside their libraries.
