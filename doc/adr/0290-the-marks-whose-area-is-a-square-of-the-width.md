# 0290 — The marks whose area is a square of the width, and the two that were disappearing

**Status.** Accepted.
**Context.** `doc/todo/11`'s remaining §10.7.4 item — "the cap a substitute does not draw under the
quantum", ADR 0268's one deliberate loss — and a second mark nobody had measured, found while
building the ladder for the first.

## The sentence, and the two marks it was not reaching

ISO 32000-2 §10.7.4:

> A shape shall be scan-converted by painting any pixel whose half-open square region intersects
> the shape, no matter how small the intersection is. This ensures that no shape ever disappears as
> a result of unfavourable placement relative to the device pixel grid …
> The area covered by painted pixels shall always be at least as large as the area of the original
> shape. This rule applies both to fill operations and to strokes with non-zero width.

ADR 0226 and ADR 0268 answered that for a stroke's **swept body**, whose area goes *with* the
width: widening a band by `k` multiplies its area by `k`, so a substitute one device pixel wide at
`1/k` of the alpha puts down exactly what the document asked for. Every other mark a stroke makes
has an area that goes as the **square** of the width — §8.4.3.3's two projecting caps and
§8.5.3.2's dot — and for those the body's alpha restores only one factor of `k`. ADR 0268 measured
that, butt-capped the substitute, and wrote the cap off.

`render-quorra/examples/sub_pixel_marks` grew a fifth and a sixth section to say what that cost.
Total ink over the raster against the mark's own area, on the processor, at scale 1:

```text
  cap       angle   length   width      before     after    its own area
  Round         0     0.15    0.50      0.0000    0.2353         0.2713
  Round         0     0.50    0.50      0.2510    0.4392         0.4463
  Round         0     1.00    0.50      0.5020    0.6745         0.6963
  Round        30     0.50    0.50      0.1882    0.3765         0.4463
  Square        0     0.15    0.50      0.0000    0.2510         0.3250
  Square        0     0.50    0.50      0.2510    0.4863         0.5000
  Square       30     1.00    0.50      0.5020    0.7216         0.7500

  §8.5.3.2's dot, diameter     before     after    its own area
                      0.10     0.0000    0.0157         0.0079
                      0.20     0.0000    0.0314         0.0314
                      0.50     0.2510    0.1882         0.1963
                      1.00     0.7529    0.7529         0.7854
```

Three findings in that table, and only the first was on the list:

1. **A rule as long as it is wide lost 44% of its area under round caps and 50% under projecting
   square ones**, and a rule *shorter* than its own width lost all of it — `0.15` long and `0.5`
   wide drew **nothing at all**, which is the disappearance the clause forbids by name.
2. **§8.5.3.2's dot vanished outright at 0.1 and 0.2 of a device pixel.** Nobody had measured it.
   It is the same shape as a cap with no rule under it — "a filled circle centred at the single
   point" — and a circle of the line's width is `π w² / 4`, which at a fifth of a pixel is three
   hundredths of one. It was `silent`: no report, no gate, no witness named.
3. **The same dot was 28% *heavy* at half a pixel**, because `tiny-skia` rounds a shape crossing
   one of its sample lines up to a quarter of a row. Removing a promotion is what says a rule of
   this kind is not simply pushing ink at the problem — the same evidence ADR 0226 offered.

## What was done

`pdf_render::enlarged_mark` states the whole family in one place: a mark whose area is a square of
the line's width, at a width under the device's quantum, is stated at `substitute_width`'s width
with an alpha of `(w / W)²`. That is exact at every width, it is the identity ADR 0268 already
uses with the exponent the shape actually has, and it is the identity at the quantum itself, where
nothing is widened and the rasteriser draws what the document wrote.

Three marks reach it, and they are all of §8.4.3.3's and §8.5.3.2's:

- **The caps**, `pdf_render::sub_pixel_caps`, built as Table 53's own shapes — a semicircular arc
  of the line's width, or a square continuing "for a distance equal to half the line width" — in
  the shared crate, for `crate::degenerate`'s reason: a decision either backend can make alone is a
  decision neither has made.
- **§8.5.3.2's dot** and **a zero-length dash's mark**, which `crate::degenerate` already builds;
  they need no new geometry, only the substitute's diameter and the alpha.

**The cap is a separate mark rather than a wider stroke, and that is the whole trick.** Table 53's
projecting caps lie *outside* the butt-capped body — that is what "beyond the endpoint" means — so
the two draws are disjoint and their ink adds, where a stroker's own round-capped outline at `W`
would carry the cap at the *body's* alpha and overstate it by `W / w`. ADR 0268's measurement of
that overstatement stands and is the reason this is two marks and not one.

## What it costs, measured

- **The oracle**: every verdict count identical — agrees, contradicted, ambiguous, our geometry,
  reference geometry, not comparable, no render, all unmoved over 1794 pages. **36 of the per-page
  lines moved**, 35 of them in the third or fourth decimal place. The one that moved visibly is
  `issue12295.pdf` page 1, the corpus's extreme, and it moved **towards** the references: worst
  mean 5.61 → 5.55, worst tile 28.43 → 26.87, ssim 0.7486 → 0.7494.
- **`doc/todo/00`'s step-7 ink sweep**, over all 786 ambiguous pages, before and after: **49 rows
  moved and every one of them up**. The negative tail is unchanged — twenty at or past −1, the same
  names in the same order — with `issue12295.pdf` −2.956 → −2.823 and `issue14297.pdf` −1.150 →
  −1.145.
- **The cross-backend gate**: `957 pages compared: 915 agree, 37 differ, 5 refused, 17 not
  comparable`, identical before and after, which is worth a sentence rather than a shrug: the
  device draws no round cap at all (below), so a page whose sub-pixel rules are round-capped now
  has one backend drawing a mark the other does not — and no page in the corpus is moved past the
  bound by it.
- **The corpus gate**: unchanged, and the whole workspace suite passes.
- **Instructions**, `callgrind_rasterise`, 20 rasterisations: page 101 of ISO 32000-2 itself
  **+0.19%** (5.544e9 → 5.554e9), and `issue12295.pdf` page 1 **+146.8%** (21.30e9 → 52.57e9).
  That second number is the honest cost and it is priced in `doc/todo/11`: the page states 65 859
  sub-pixel round-capped strokes and every one of them is now a second `scan::fill` of a curved
  shape, which the profile confirms — `fill_path_impl` 1.41e9 → 5.41e9 and `push_cubic` from
  nothing to 2.45e9. An ordinary page pays the width comparison and nothing else. The one
  optimisation taken is the one that costs no clarity: the cap shapes are read from the path the
  display list already holds, and the conversion back from the rasteriser's own type is paid only
  where the dasher has cut new ends (0.8% of the page's total).

## The residual, which is the raster's rather than the construction's

`issue12295.pdf` page 1 is where this is visible, and `pdf-model/examples/sub_pixel_width_census`
is the instrument that says why. All 65 859 of its sub-pixel strokes are **0.1366 of a device pixel
wide**, near-black, and round-capped, so their caps are **2170.93 device pixels** of geometry —
**1.14 levels of 255** over the page. The page's ink rose by **0.133** of a level, 7.547 → 7.681.

An eighth of what the geometry states, and the arithmetic says where the rest went: a cap at that
width is `0.0073` of a pixel of ink spread over the few pixels the substitute covers, which is
about **half a level of 255 each**, and half a level is what an eight-bit raster rounds away. The
mark no longer disappears — that is the clause's own requirement and it is met — but the amount
that lands is what the raster can hold rather than what the shape states. Recovering the rest means
*adding* the cap's coverage into the same mark as the body instead of compositing a second draw at
its own alpha, which is one draw rather than two and would be cheaper as well as exacter; it needs
the subpath's arc length and a second construction for a path with joins, and it is written down in
`doc/todo/11` rather than taken here.

**Our own 8× ladder for that page is 6.934 before and after**, which is the check rather than a
spare number: at that scale the strokes are no longer sub-pixel and none of these rules may touch
them. The page therefore sits 0.75 of a level above its own geometry where it sat 0.61 before, and
0.13 of that is this change. Every reference sits 3 to 7 levels above it, which is
`AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY`'s standing subject.

## What the ladder found about the other backend, which is not this tree's to fix

Two readings of quorra, both reproducible with `sub_pixel_marks` and both written up as section 21
of `doc/QUORRA_FEEDBACK.md`:

- **It draws no round cap at all, at any width.** A 40-unit rule 5 units wide with round caps
  carries 200.157 of ink where its area is 219.635, which is the butt-capped answer to the last
  digit. Square caps are right to a tenth of a per cent.
- **It flattens a small circle into a polygon inscribed in it**: §8.5.3.2's dot at one device pixel
  reads 0.5020 against `π/4` — the inscribed square exactly — and at two pixels 2.8235, the
  inscribed octagon.

`render-quorra/tests/sub_pixel_coverage.rs` therefore holds **the processor alone** to the two rows
those two readings cover, with the numbers and the reason in the test's own comment. That file
exists to gate the *clause* on both backends; asserting a row the device cannot draw would ratchet
a defect rather than a requirement, which is the mistake its header records having avoided once
already. Both come back the moment either row draws its area.

## What was considered and not done

- **Keeping the stroker's cap on the widened substitute.** ADR 0268 measured it: overstated by
  `W / w`, and 66% over its own 8× limit on `issue12295.pdf`. Unchanged, and it is why the cap is
  its own mark.
- **A cap at the geometric mean of the two widths**, `√(wW)/2`, drawn in the *same* fill as the
  body at the body's alpha — the algebra works out to exactly the cap's area and it needs one draw.
  Refused: at `w = 0.03` that cap is a half-disc of radius 0.087 of a pixel, whose area is under
  `tiny-skia`'s own coverage quantum, so the construction hands the mark straight back to the
  quantum this module exists to get around.
- **Restoring the cap only for a subpath shorter than its own width**, which is where the loss is
  largest. Refused for the discontinuity: at `L = w` exactly the cap is 44% of the mark either
  side of the boundary, and a rule that draws it at 0.99 `w` and drops it at 1.01 `w` is a step
  where the geometry has none.
