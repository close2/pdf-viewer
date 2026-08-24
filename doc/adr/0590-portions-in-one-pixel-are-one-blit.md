# 0590 — Portions that share a device pixel are summed into one buffer and blitted once

**Status.** Accepted. Closes `doc/todo/11` item 7's remainder and departure (1)'s multi-rectangle
population.

## Context

ADR 0583 read §11.6.2 forwards and found that a path stating several axis-aligned rectangles is
**one object** whose subpaths are portions of it, so §11.3.7.3's union — the arithmetic item 7 had
been waiting on item 5 to answer — governs two *objects* and cannot arise here at all. It then paid
the half of the population where no device pixel receives two portions: each is drawn as its own
mark at §10.7.4's exact area, for a fill and for a clipping region alike, because drawing them one
at a time composites nothing with anything there.

It left the other half with a price written down rather than built:

> What is left is the sharing half, 505 of 3924 in that corpus, and its price is *not* item 5's
> rasteriser: it is one coverage buffer per mark with the portions' areas **summed** into it and the
> paint blitted once, which is `scan::intersected`'s shape already (ADR 0355) and would cost about
> what that cost.

That price is right, and this decision is what it took to spend it. The census reproduces exactly:
`pdf-model/examples/rectangular_path_census` over the pdf.js corpus's 958 readable first pages at
scale 1 reads 223 545 fills, 12 987 one rectangle, 3419 several with no shared pixel, **505 several
sharing one**, 3084 with a rectangular subpath declined. They lie on **23** pages.

## The construction, and why it is two clauses rather than one

A pixel two portions reach may not receive two compositing steps — that is §11.6.2 — and each
portion's own boundary in that pixel must still be measured at the area it covers, which is
§10.7.4's third sentence and the whole of item 7. **The two requirements are independent**, and one
buffer meets both: the portions' exact areas go into it and the paint is blitted through it once.

The buffer is not new. `scan::intersected` has held one since ADR 0355, where it exists so that a
**clip** can meet a mark by `min` instead of by the product `tiny_skia::PixmapMut::fill_path`
applies. What had to change is the sentence that decides when it is built:

- it declined outright wherever there was no clipping region — `Clip::Unclipped` and a bare soft
  mask went straight to the library — and §11.6.2 asks for the same buffer with **no clip at all**;
- `is_a_set` is a *cost* decline, correct while the buffer exists only for the clip's sake: where
  the clip is already 0 or 255 under the mark, `min` and the product are the same function and the
  cheaper path is also the right one. It may not be applied to a mark whose own portions share a
  pixel, because no property of the clip answers for what the *mark* needs.

So `intersected` now takes the `Clip` itself rather than a pre-extracted triple, asks it for its two
factors, and builds the buffer when **either** clause wants it. `Clip::Unclipped` and `Clip::Value`
gained the scratch buffer the other two variants already carried, which is the type saying the same
thing: a buffer that answers to the mark cannot belong to the clip's presence.

`scan::Exact` gained a fourth variant rather than a flag. `Exact::Several` and `Exact::Shared` admit
*different constructions* — the first may be drawn one rectangle at a time through
`tiny_skia::PixmapMut::fill_rect`, the second may not be drawn that way at all — so the variant is
what keeps a caller from reaching the forbidden loop, and the fallback where `intersected` declines
is the single supersampled conversion rather than the loop.

## What a shared pixel is covered by, and why the summing is two passes

The portions are one object, so what covers a pixel is the area of their **union** there — not
§11.3.7.3's union *function*, which is what compositing them one at a time would apply.
`pdf_render::device_rectangles` already requires their interiors to be pairwise disjoint (it
declines an overlap outright, because the two fill rules answer that case differently), so the area
of the union is the plain **sum** of the areas and no inclusion–exclusion term survives. It is
capped at the whole pixel, which the disjointness makes arithmetic rather than a guard.

**The rounding is where this round found something ADR 0583 had not priced.** Accumulating each
portion's own *rounded* level is a level or two out per shared pixel — `0.3 + 0.3` of a pixel is
`77 + 77 = 154` where `0.6` is 153 — and a coverage rounded away is the whole subject of item 7. So
`scan::mask_shared_rectangles` is two passes:

1. each portion at its own exact area, which is the complete answer for every pixel only one of them
   reaches, and which keeps ADR 0476's interior **run** — without it, asking the closed form per
   pixel costs a third of a page's rasterisation;
2. exactly the pixels two footprints have in common, where the total over *every* portion is
   computed in one addition and rounded once.

The second pass's pixel set is bounded by the portions' shared **boundary** rather than by their
area, and the pairwise walk is `share_a_device_pixel`'s own question asked again, quadratic in a
count `pdf_render::RECTANGLES_PER_PATH` bounds at 32.

`two_portions_in_one_pixel_are_summed_and_rounded_once` is the discriminator and it separates every
candidate at once. An object covering a device row 0.3 by one portion and 0.3 by the other carries
**153** of 255; §11.3.7.3's union of the two would give 130, taking the larger of them 77, and
accumulating their two roundings 154.

## What it moved

Both arms were built from the sources — ADR 0583's own lesson, that an `if true { return }` lets the
optimiser delete the new code and reads a per cent low. Machine load was 1.0 for the pixel arms and
rose to 71 under a neighbour's gates for the second callgrind arm, so **no timing figure was
taken**; every number below is a raster value or an instruction count, neither of which load moves.

**`raster_digest` over the pdf.js corpus: 22 of 958 first pages move, and every one of them is a
page the census named before a line was written.** No other page moves at all. The one listed page
that does not move is `issue12963.pdf`, whose three such fills land where the quarter and the area
agree. That confinement is the strongest evidence here that the construction is reached exactly
where the clause's condition holds and nowhere else.

The pages themselves, looked at rather than counted (trap 1): `issue8187.pdf` is a **barcode**, and
its bars come out at the sub-pixel levels their own widths imply instead of quantised to a quarter —
mean 1.02, 5.29% of pixels differing, max 26 levels. `issue840.pdf` and `issue11913.pdf` are
ordinary pages of text and artwork, indistinguishable at a glance and differing only in edge levels;
a share of the moving marks is text, because a glyph whose outline is two axis-aligned rectangles is
a two-subpath fill like any other, and two of its rectangles can perfectly well share a pixel.

```text
  before → after, our own raster (raster_compare)
  issue8187.pdf    mean 1.0249  worst tile 4.46  max 26  differing 5.2875%  ssim 0.99705
  issue840.pdf     mean 0.0079  worst tile 0.31  max 37  differing 0.0535%  ssim 0.99995
  issue11913.pdf   mean 0.0232  worst tile 0.43  max 58  differing 0.1449%  ssim 0.99990
  160F-2019.pdf    mean 0.0096  worst tile 0.38  max 46  differing 0.0484%  ssim 0.99994
  issue1350.pdf    mean 0.0114  worst tile 0.30  max 75  differing 0.0486%  ssim 0.99996
```

`callgrind_rasterise`, `RAYON_NUM_THREADS=1`, twenty rasterisations:

```text
  ISO 32000-2 p101 (text, no such fill)  5,414,365,181 -> 5,417,771,847   +0.063%
  issue8187.pdf  p1   (6 of 14 fills)       45,230,325 ->    30,640,987  -32.256%
  issue11913.pdf p1   (96)               5,387,032,288 -> 5,372,623,208   -0.267%
  160F-2019.pdf  p1   (45)               3,290,771,464 -> 3,303,278,333   +0.380%
  issue840.pdf   p1   (97)               5,405,390,201 -> 5,456,728,448   +0.950%
```

`issue8187.pdf`'s −32% is the same finding ADR 0476 recorded and for the same reason: a rectangle
handed to `tiny-skia`'s rectangle scan converter is cheaper than a path handed to its supersampled
one, and a page that is almost nothing but such rectangles pays almost nothing. The +0.95% at the
other end is the buffer's clear, compose and blit over each of 97 fills' own reach, which is ADR
0355's cost on the pages that ask for it; it is recorded rather than optimised, because the page
carrying it is one of twenty-two and the alternative is a wrong coverage.

**No gate's verdict moved, and both arms were run rather than one being quoted.** The reference
oracle reports 983 agrees, 65 contradicted, 832 ambiguous, 3 our geometry, 2 reference geometry, 42
not comparable and 18 no render before and after; **21 of its 966 per-page lines moved**, all of
them `ambiguous`, none across a bound, the largest being `issue8187.pdf` at mean 18.89 → 18.73 and
ssim 0.7901 → 0.7921 — toward the references. The cross-backend gate is 933 agree / 22 differ both
ways with the same names, two of its differing lines moving toward the device. `doc/todo/00` step
7's ink sweep over all 768 ambiguous pages moves 25 rows, 20 up and 5 down, by at most 0.062 of 255,
with the negative tail the same 19 names in the same order and ADR 0433's five heads reproduced to
the thousandth.

**One optimisation was tried, measured and removed**, which is worth a line because the shape
recurs. `scan::fill` — the call every glyph on a page of text makes — now reaches `intersected`
unconditionally where it used to guard on `clip.composable().is_some()`, and the guard is exactly
that function's own first two declines for an `Exact::Unknown` mark, so restoring it is free and
looks like an obvious saving. Measured, it *costs* 13 000 instructions of 5.4 × 10⁹ on page 101
rather than saving any: one call frame per fill is below what a page of text can measure, and the
+0.063% above is codegen rather than the branch. The guard is not in the tree.

## Consequences

- `doc/todo/11` item 7's remainder is closed, and `doc/todo/_scan-conversion.md`'s departure (1) no
  longer has a multi-rectangle population. What still carries `tiny-skia`'s quantum is every shape
  that is not axis-aligned rectangles at all — a glyph's curves, a diagonal, a stroke's outline that
  is not one rectangle.
- **This is `render-cpu`'s alone**, like ADR 0476 and for the same reason: `render-quorra` and
  `render-gpu` track a fraction to a level of 255 natively and neither reads `pdf_render::edge`. The
  cross-backend gate should therefore move in the direction of agreement, as it did for ADR 0583's
  half.
- Overlapping rectangles stay declined where they were: §11.6.2's own sentence names that case and
  the two fill rules answer it differently, so a decomposition would have to carry a winding number
  per cell. Nothing here changes that.
- `Clip` now carries a scratch buffer in every variant. The cost is one word in a `Copy` enum whose
  largest variant already held three, so its size is unchanged.
