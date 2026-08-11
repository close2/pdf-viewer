# ADR 0268 — The hairline paints the projection, not the line

Status: accepted, 2026-08-11. Session 432. The residual ADR 0226 named and priced, re-derived
rather than inherited — and both halves of the price were wrong.

## What ADR 0226 left, and what it said the answer would cost

> **A diagonal sub-pixel rule.** `22060_A1_01_Plans.pdf` is the whole corpus's worst case for it
> and is untouched. The substitution needs a pixel *line* to stretch into and a slanted band's
> pixel run is a staircase; what would answer it is a per-scanline coverage span rather than a
> rectangle, which is a different construction and not this one.

Three claims: that the case is a *disappearance* like the two ADR 0226 closed, that the named page
is its witness, and that answering it means a scan converter of our own. The first is false, the
second is false about the page, and the third does not follow from either.

## What §10.7.4 asks for a shape whose zero-extent axis is not an axis

The subclause states one construction and two guarantees:

> A shape shall be scan-converted by painting any pixel whose half-open square region intersects
> the shape, no matter how small the intersection is. This ensures that no shape ever disappears
> as a result of unfavourable placement relative to the device pixel grid, as might happen with
> other possible scan conversion rules. The area covered by painted pixels shall always be at
> least as large as the area of the original shape. This rule applies both to fill operations and
> to strokes with non-zero width.

**The construction contains no axis.** The pixels a shape meets are a set; for a turned band that
set is a staircase, which is a harder set to *enumerate* and not a different requirement. What
needed a pixel line was ADR 0226's own substitution — one rectangle carrying one coverage — rather
than anything the standard says.

**And a diagonal splits the two guarantees apart**, which is the finding. Measured on
`render-quorra/examples/sub_pixel_marks`, whose fourth section this session added: a 200-unit band
at seven angles and six thicknesses, as a fill and as a stroke, total ink over the whole raster
against the band's own area.

### Guarantee one is not at risk for a diagonal, and cannot be

`tiny-skia` supersamples four times per pixel row at each sub-row's centre, so a band lying
*between* two sample lines crosses none of them and vanishes. A band that is not parallel to them
crosses one every `1 / (4 tan θ)` pixels of its length, and over any appreciable length it therefore
crosses many. The measurement says so outright — a filled sliver 0.05 of a pixel thick, error
against its own area on the processor:

```text
  angle        5°       15°      30°      45°      60°
  fill      +2.3%    −2.7%    −2.1%   +255%    −5.3%
```

against **0 ink** for the axis-aligned one before ADR 0226. So a diagonal **fill** is owed nothing
and this ADR does not touch one. (The +255% at exactly 45° is the same converter's other quantum
and is a *promotion*: it reads a constant 35.5137 for 0.05, 0.1 and 0.2 alike, which is a quarter of
a pixel per row over the 141.42 rows the band spans. Erring heavy is the side the second guarantee
asks for.)

### Guarantee two fails, and it is the hairline rather than any quantum

`tiny-skia` draws a stroke under a pixel wide as a hairline with the paint's opacity scaled by the
width. **Its hairline lays one pixel down per step along the line's longer device axis**, so the ink
it puts on the page is the line's *projection* times the width where the area is the line's *length*
times the width. The deficit is therefore `1 − cos θ` with `θ` measured from the nearer axis, it is
exact, and — the part that makes it a defect rather than a rounding — it does not depend on the
thickness at all:

```text
  a 200-unit rule, ink against its own area, on the processor, before
  angle        0°       5°      15°      30°      45°      60°      90°
  0.05      −5.9%   −6.5%    −9.4%   −18.8%   −33.4%   −18.8%    −5.9%
  0.10      +2.0%   −2.8%    −5.7%   −15.5%   −30.7%   −15.5%    +2.0%
  0.20      +2.0%   −0.8%    −4.0%   −13.7%   −29.3%   −13.7%    +2.0%
  0.50      +0.4%   −0.4%    −3.4%   −13.4%   −29.0%   −13.4%    +0.4%
                            └ cos 15° = 0.9659, cos 30° = 0.8660, cos 45° = 0.7071
```

The bottom row *is* the cosine, to three digits. That reads directly against "[t]he area covered by
painted pixels shall always be at least as large as the area of the original shape", in the
direction the sentence forbids.

## The rule

**A stroke thinner than one device pixel that the exact substitution cannot take is drawn as the
same path stroked one device pixel wide, with the width it gave up carried in the paint's alpha.**

`pdf_render::substitute_width` states the width; `render-cpu`'s `draw_rule_at_one_pixel` draws it;
the device is already right, so this is the *oracle* being brought up to it and nothing in the
display list moves (trap 2).

Three things follow and each is why this is the construction rather than a scan converter:

- **The ink is exact at every angle**, because widening by a factor `k` multiplies the device area
  by `k` and dividing the alpha by `k` divides it back. That identity holds for any transform and
  any angle, and it is the whole of the arithmetic.
- **It is §10.7.4's own construction.** The clause's answer for a mark too thin to measure is a run
  of *whole pixels* — NOTE 1 and the EXAMPLE beside it say so for the degenerate case and
  `pdf_render::collapsed` has drawn it since ADR 0154. A rule one device pixel wide is that run;
  the fractional alpha is the anti-aliasing departure, which is where the coverage goes.
- **Nothing is snapped**, which is what `doc/todo/_scan-conversion.md` demands of anything touching
  the pixel grid. The band stays centred on the path at the fractional position the document put
  it. §10.7.5's promotion under `/SA` multiplies the *ink* by `one_pixel / w`; this divides the
  alpha by the same factor instead, so the two are opposite operations rather than neighbouring
  ones.

Coverage riding in alpha is ADR 0226's argument unchanged — §11.3.7.1's "the alpha values that
control the compositing process shall be defined as the product of shape and opacity", with
§11.3.7.2's NOTE 1 naming anti-aliased coverage as the first factor — and its one exception is
honoured by the same gate: `carries_coverage_as_alpha` refuses §11.4.6's knockout group and refuses
the mode where this rasteriser is not anti-aliasing at all.

### Why the exact substitution is still tried first

Because this one is blunter, and the ordering is the whole of the relationship between them. A band
one device pixel wide spreads its ink over a pixel where a band of the true width spreads it over
`w`; and at the raster's edge the half of the wide band that falls outside is still lost, which is
exactly the fault ADR 0226 removed for the axis-aligned rule. So `sub_pixel_bands` keeps the cases
it can measure exactly and this takes everything else.

### The width is stated from the transform's *smaller* stretch

`pdf_render::thinnest_line` is `1 / max_stretch`, which is §8.4.3.2's reading — how wide a stroke
can *become*. The substitute needs the other one: a band that is at least a whole pixel across
whichever way the path runs, or it is handed straight back to the quantum this exists to get around.
So `substitute_width` is `1 / min_stretch`, `Transform::min_stretch` is the smaller singular value,
and for a similarity — every page transform — the two are equal to the last bit. Where they differ
the substitute is wider along the stretched axis, which costs sharpness and costs no ink, by the
identity above.

### The substitute is the swept body, and the cap is what it gives up

The ink identity above is the **body's**. A cap's area goes as the square of the width, so widening
multiplies it by `(width / style.width)²` where the alpha divides it back only once, and the end is
then overstated by `width / style.width` — which for a rule a twentieth of a pixel wide is twenty
times.

That is not a bound written down after the fact: it is the version of this rule that was measured,
shipped nowhere, and replaced. `issue12295.pdf` states **65 859 strokes thinner than a device
pixel**, every one of them round-capped, **91.8% of them shorter than one device pixel** with a
median length of 0.145, so two round caps at one device pixel are `π/4` of a pixel against a body of
0.145. Ink against the page's own 8× limit of 6.934:

```text
  before the rule            8.792     +27% over the limit
  keeping the round cap     11.503     +66%
  the swept body alone       7.547      +8.9%
```

and the two backends' own gate reads 2.157, 4.275 and 1.649 on the same three. So the substitute is
butt-capped.

What that gives up is the cap's own area, `O(w²)` with `w` under one device pixel, and one case
where that is not nothing: a round-capped subpath *shorter than its own width* is a dot of area
`πw²/4` and the body replacing it is thinner still. §8.5.3.2's exactly-degenerate subpath is already
taken out and filled as a dot by `pdf_render::split_degenerate`, and the nearly-degenerate one is
where this would be answered — not in `render-cpu`. `doc/todo/11` carries it, and `doc/todo/00`'s
step-7 sweep over all 786 ambiguous pages is the evidence that no corpus page reports it.

### What it takes that ADR 0226 declined

A dash, a round cap, a curve, a polyline that turns a corner, a rotation, a shear. The dash is worth
naming because ADR 0226 declined it on a reason that has expired: the dashes are dispensed by
`tiny_skia::Path::dash`, the same function `stroke_path` would have called, rather than by a second
implementation of §8.4.3.6 in this tree. The pattern is measured along the path, so widening the
stroke does not touch it.

**A fill is still not touched at all.** ADR 0226's permanent decline — a thin triangle, a glyph stem
— is about a shape whose cross-section varies along its length, and nothing here reopens it.

## What it does, measured

`render-quorra/examples/sub_pixel_marks`, section 4, both backends, scale 1, on a 320 × 320 page,
a 200-unit rule stroked at each angle. Error against the band's own area:

```text
                      before                    after
  angle  thick     cpu     quorra          cpu     quorra
      0   0.05   −5.9%     −5.9%         −5.9%     −5.9%
      5   0.05   −6.5%     +1.5%         +8.3%     +1.5%
     15   0.05   −9.4%     −1.2%         +9.1%     −1.2%
     30   0.05  −18.8%     +0.7%         +9.5%     +0.7%
     45   0.05  −33.4%     −0.2%         −0.2%     −0.2%
     60   0.05  −18.8%     +0.7%         +9.5%     +0.7%
     90   0.05   −5.9%     −5.9%         −5.9%     −5.9%
      5   0.50   −0.4%      0.0%         +0.3%      0.0%
     15   0.50   −3.4%      0.0%         +0.2%      0.0%
     30   0.50  −13.4%      0.0%         +0.1%      0.0%
     45   0.50  −29.0%     −0.2%        −11.3%     −0.2%
     60   0.50  −13.4%      0.0%         +0.2%      0.0%
```

The 0° and 90° rows are the axis-aligned case ADR 0226 takes and are unmoved, which is the ladder
crossing from one construction to the other without a step.

**The 45° column is `tiny-skia`'s and not this rule's**, and it is measured rather than asserted: the
plain fill of a *one-device-pixel* band at exactly 45° reads **177.44 of its own 200** on this
converter, because it quantises that band's per-row run to quarter pixels. The substitute inherits
that whole and cannot be held tighter. Away from the knife edge the worst residual is +9.5%, at 0.05
of a pixel, where an eight-bit raster has one level to spend.

### And what it does to shapes that are *not* degenerate

The same constraint ADR 0226 was held to, now in the other axis. Section 3's ladder across the
one-pixel boundary is **unmoved in every row on both backends** — this rule fires only below one
device pixel and the exact substitution still takes the axis-aligned cases there — and section 4's
own 1.00 and 2.00 rows say what happens above it:

```text
  a 200-unit rule at 45°, ink against its own 200
  thickness     cpu      quorra
       0.50   88.72      99.83
       1.00  141.42     200.20
       2.00  389.17     399.87
```

**The 1.00 row is a defect this instrument found and this ADR does not pay.** `tiny-skia` chooses
the hairline for `width <= 1.0`, so at *exactly* one device pixel a turned rule still gets the
projection — 141.42 of 200 at 45°, where the fill of the same outline reads 177.44 — and this rule
stops strictly under one pixel, leaving a one-point discontinuity at the boundary. It is not this
file's subject: a mark thinner than the document said is not a mark that disappeared, and no quantum
is involved. It is written into `doc/todo/11` with its numbers and with the two things that have to
be settled before it is taken — the `0 w` stroke, which §10.7.4 exempts by name and which
`Stroke::device_width` makes indistinguishable at the rasteriser, and the blast radius, which is
every page in the corpus with an ordinary hairline.

### On a document, and it is not the one ADR 0226 named

**`22060_A1_01_Plans.pdf` was never the witness**, and counting its page said so in one run. Page
one is 136 commands: **72 sampled images**, whose combined device footprint is 1 524 354 px² on a
250 916-pixel raster, 24 fills and 40 strokes. Of those 40, twenty-six are sub-pixel and **98% of
their length lies within 5° of a device axis**, where the hairline's deficit is 0.3%. The page moves
**+0.06%** under this change and that is the correct answer for it. Its line work is §10.7.4's
*image* paragraph and ADR 0025's area averaging; `oracle.rs`, `_scan-conversion.md` and the ledger
called it "all strokes under a pixel wide" for forty-three sessions and are corrected.

**The witness is `issue11473.pdf`**, four hatch swatches painted with `/PatternType 1` cells whose
whole content is `0.3985 w` strokes — one axis-aligned grid and three diagonals. `AMBIGUOUS_SUB_PIXEL_LINE_WORK`
already records that its two reference ladders converge on **0.752 to 0.760** of 255 and that ours
started at 0.678. Ink over the page at 1×:

```text
  ours before 0.6768   ours after 0.7566   two-ladder limit 0.752 to 0.760
```

Ten per cent under the geometry to on it, and the picture says it too: the three diagonal swatches
are visibly denser and the grid swatch is unchanged.

## What it costs

`callgrind_rasterise`, twenty renders, instructions:

```text
                                        before              after
ISO 32000-2 page 101             5,531,973,848      5,519,354,783    −0.23%
22060_A1_01_Plans.pdf page 1    87,128,779,309     87,210,085,056    +0.09%
tiling-pattern-box.pdf page 1      953,577,844      1,040,570,781    +9.12%
issue11473.pdf page 1              760,136,232        896,067,975   +17.88%
issue12295.pdf page 1           15,957,037,410     21,236,216,032   +33.08%
```

What the pages that pay are paying for is the stroker: a hairline allocates no path and an outline
does, and the four questions in front of it were already as cheap as they go. So the cost is
proportional to how much of a page is sub-pixel stroking, and the table is that statement four
times — page 101 of the standard does not move at all, the plans drawing has 26 such strokes and
moves 0.09%, the hatch pages have nothing else and move 9% and 18%, and **`issue12295.pdf`'s 65 859
of them cost 33%**.

That is the honest ceiling and it is on the page the rule does the most for: 27% over its own 8×
limit to 8.9% over, and the two backends 2.157 apart to 1.649. There is no cheaper way to have the
outline, and only one outline is built — a path that fails the axis-aligned questions goes straight
to the general construction rather than being stroked twice.

`22060_A1_01_Plans.pdf` is the corpus's most expensive page and it pays 0.09%, for the reason its
own paragraph above gives: it has 26 sub-pixel strokes and 72 images.

## What the gates said

Every one of them, before and after, because this changes the oracle.

- **corpus** 974 documents, **68** incomplete, the same set. 8 locked, 2 encrypted beyond us, 5
  pageless, 0 unopenable, 0 slow.
- **oracle** 1794 pages, 1690 complete: agreeing **905**, contradicted **68**, ambiguous **786**,
  our geometry 1, reference geometry 2, not comparable 14, no render 18 — **every count identical
  before and after**, which is what a rule that moves ink within a page's own tolerance looks like.
  No ratchet in that gate moved.
- **quorra vs the CPU oracle** 957 pages, **910 agree / 36 differ → 911 / 35**, 11 refused and 17 not
  comparable unmoved. One document left, `issue12810.pdf`, and it is the corpus's real witness for
  this rule: 34 787 sub-pixel strokes, 25 406 of them not axis-aligned, **62.6% of their 173 316
  pixels of length within 40° to 50° of an axis**, which is 19.4% of the page's stroke ink weighted
  by length and all of which quorra was already drawing. Its own ink goes 5.0782 → 5.0426 against a
  4× rung of 5.017. `DIFFERS_AT_THE_EDGES` is re-ratcheted 24 → **23** with that argument.
  `issue12295.pdf` stays on the list and moves **2.157 → 1.649**.
- **text** 99.2% (24 003 of 24 187), 23 below the floor — unmoved, both gates. **dates** 1514 of
  1545. **XMP** 318 read, 1 refused, 3191 properties. **JPEG 2000** 14 byte-identical.
- **workspace** 1568 → **1572** tests, 11 skipped. **conformance** 6364 → **6373** citations,
  595 → **596** quotations, 0 cited clauses owing a review, 875 ledger rows and every status
  unmoved.
- **`doc/todo/00`'s step 7**, our ink minus the lightest reference's over all 786 ambiguous pages:
  **twenty names at or past −1 before and after, the same twenty**, and every other entry moves
  *up* — `issue16038.pdf` −5.758 → −5.734, `issue7339_reduced.pdf` −0.470 → −0.435,
  `ThuluthFeatures.pdf` −0.148 → −0.134, `issue11473.pdf` and `tiling-pattern-box.pdf` off the
  negative tail altogether. **One entry moves down and it is `issue12295.pdf`, −1.712 → −2.956**,
  which is the page above: its own 8× limit is 6.934, the lightest reference is `mupdf` at 10.504,
  and moving toward a geometry every reference overpaints is moving away from all of them.
  `AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY` is the group that already says so and it now says it with
  a smaller number.
