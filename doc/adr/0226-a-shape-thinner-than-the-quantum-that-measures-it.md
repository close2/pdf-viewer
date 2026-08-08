# ADR 0226 — A shape thinner than the quantum that measures it

Status: accepted, 2026-08-08. Session 389. The two shapes `doc/todo/11` had open since the
hundred-and-eighty-sixth session, both of them the **oracle's** and neither of them the device's.

## What §10.7.4 promises, and the two ways this tree was failing it

> A shape shall be scan-converted by painting any pixel whose half-open square region intersects
> the shape, no matter how small the intersection is. This ensures that no shape ever disappears
> as a result of unfavourable placement relative to the device pixel grid, as might happen with
> other possible scan conversion rules.

Both backends anti-alias, which is a departure §10.7.1's NOTE licenses and which replaces the
clause's *paint the pixel* with **coverage proportional to area**. That replacement is what has to
hold. A twentieth of a pixel drawn at a twentieth of the ink is neither the clause's answer nor
nothing; a twentieth of a pixel drawn at **nothing** is the disappearance the clause's stated
purpose forbids, reached by a different road.

ADR 0154 closed the case where the shape has *no* area. This one is the case where it has an area
and the rasteriser cannot express it.

### The mechanism, and it is one mechanism reached by two routes

`tiny-skia`'s anti-aliased scan converter **supersamples four times per pixel row and takes each
sub-row's sample at its centre** — device y = 0.125, 0.375, 0.625, 0.875 — and quantises a run
along x to quarter-pixel steps. So its smallest non-zero coverage is a sixteenth of a pixel, a
shape lying between two sample lines contributes nothing at all, and a shape crossing one is
rounded **up** to a quarter of a row.

1. **A fill.** An 80-unit rule at scale 1, ink against its own area:
   `0.05 → 0`, `0.1 → 0`, `0.2 → 0.2471`, `0.5 → 0.4980`. Two of the five gone.
2. **A stroke.** `tiny-skia`'s painter draws one under a pixel wide as a **hairline** with the
   paint's opacity scaled by the width. That conserves the ink and approximates the placement: the
   mark is smeared symmetrically about the path whatever fraction of a pixel the path lies at, so
   a rule within half a pixel of the raster's edge loses the half of its smear that falls off.
   0.0549 of an expected 0.1 at the edge; 0.0941 in the interior, where the exact answer for a
   rule at y 300 is 0.03/0.07 and the hairline gives 0.047/0.051 *at every placement*.

The graphics device has neither. It answers 0.0510, 0.1020, 0.2000, 0.5020, 1.0000 to the five
slivers and 0.0980 to 0.1020 to the five rules — within 2% of every area.

### Which makes it a defect in the instrument, and that is why it was worth a round

`render-cpu` is this project's correctness oracle and `render-quorra/tests/corpus.rs` calls a
difference between the two backends *quorra's* by construction. On a page of sub-pixel line work
the backend carrying the right ink was the one being accused. `issue16038.pdf` is the witness: the
two backends sat 6.5359 apart on a page whose whole subject is a 0.53-pixel rule.

## The rule

**An axis-aligned rectangle thinner than one device pixel is drawn as the whole device pixel line
it lies in, painted at the coverage its own area there implies** — and where it straddles a
boundary, as one such substitute per line, each carrying that line's share.
`pdf_render::sub_pixel_bands` computes it; `render-cpu` is the only caller, because the device is
already right (trap 2 the other way round: a display-list change would have moved both).

For an axis-aligned rectangle this is not an approximation of proportional coverage but exactly
it, in both axes at once: the substitute's ink in pixel (i, j) is the rectangle's area inside pixel
(i, j). Along the axis it is *not* thin the shape keeps its own extent and the rasteriser
anti-aliases the ends as before — only the axis where the coverage was being lost is stretched,
which is `pdf_render::collapsed`'s own reason for snapping one axis only.

**Nothing is snapped and nothing is promoted.** A 0.05-thick rule draws 0.05 of a row, where
§10.7.5's stroke adjustment would have drawn a whole one. That is the sentence
`doc/todo/_scan-conversion.md` demands of anything that touches the pixel grid, and this rule
answers it by not touching it.

### Coverage becomes alpha, and the standard says when that is legitimate

The substitute is a shape the rasteriser *can* measure, carrying a coverage it cannot. The only
place left to put that coverage is the paint's alpha, and §11.3.7.1 says the two are one quantity:

> As stated earlier, the alpha values that control the compositing process shall be defined as the
> product of shape and opacity

with §11.3.7.2's NOTE 1 naming anti-aliased coverage as the first of those two factors — "when
such objects are rasterized to device pixels, the shape values along the boundaries can be
anti-aliased, taking on fractional values representing fractional coverage of those pixels".
A raster with one alpha channel per pixel carries exactly that product, so folding a band's
coverage into it is the clause's own arithmetic and every blend function reads `αs` without being
able to tell which factor it came from.

The NOTE's warning is the exception and it is honoured by name: where shape must stay
distinguishable from opacity the substitution is wrong, and this backend has one such place —
§11.4.6's knockout group, which `tiny-skia` states as Porter-Duff `Source`. There a scaled alpha
would leave a partly transparent pixel where a partly covered one is meant, so a sub-pixel shape
inside a knockout group keeps whatever the rasteriser already gave it.

**And the rule does not fire where this rasteriser is not anti-aliasing at all.** Its whole warrant
is the departure above; with `CpuRasterizer::with_anti_alias(false)` the clause's own answer for a
sub-pixel shape is a *whole* covered pixel and a fractional alpha is neither that nor what the
caller asked for. That mode is one test's knob today and the condition costs it nothing; what it
buys is a rule that is true as stated rather than true of one configuration.

### A stroke reaches the same rule by being outlined

A sub-pixel stroke whose path is made of straight axis-aligned rules is converted to the fill of
its own outline, which is the rectangle the document's own width and coordinates state, and takes
the rule above. §10.7.5 is untouched by that and the boundary is worth stating because it is the
one `doc/todo/_scan-conversion.md` polices: **that clause's first requirement moves a stroke's
coordinates onto the grid and is conditional on `/SA`, and nothing here moves anything.** A
0.1-unit rule draws 0.1 of a row at the fractional position the document put it.

### What it declines, and why each

- **Anything but an axis-aligned rectangle.** The exactness rests on the cross-section being
  constant along the length; a thin triangle's is not. **A glyph stem is what makes this a rule
  rather than a caution** — contours a fraction of a pixel wide are what small text is made of, and
  a uniform coverage across one is worse than what the rasteriser already does.
- **A transform that is not axis-preserving.** Under a rotation or a shear the run of pixels a thin
  band passes through is a staircase and there is no pixel line to stretch into. `collapsed`
  declines the same case for the same reason.
- **A path with one subpath left over.** A fill's subpaths share a winding rule, so removing one
  changes what the others enclose. Every subpath or none.
- **Two subpaths meeting in one pixel line.** Separate draws composite as `1 − (1−a)(1−b)` where
  coverage inside one scan conversion *adds*, so two parallel sub-pixel rules sharing a row would
  come out light — and what the rasteriser does with them is not a disappearance. Two
  *perpendicular* bands need no such guard: at their crossing the exact union of a row and a column
  of coverage `a` and `b` is `a + b − ab`, which is what compositing them gives.
- **The even-odd rule where the thin axes differ.** §8.5.3.3.3 leaves a hole where two crossing
  rectangles meet and separate draws cannot express one.
- **A dash.** The dashes are dispensed by `stroke_path` itself, and dashing here would be a second
  implementation of §8.4.3.6 beside the one the library already runs.
- **A round cap**, and anything else whose outline is not a rectangle: `sub_pixel_bands` answers
  `None` for a shape it cannot measure exactly, so the hairline draws the whole rule rather than
  half of it.

## What it does, measured

`render-quorra/examples/sub_pixel_marks`, both backends, scale 1, page 100 × 320.

```text
a filled sliver, 80 units long          before            after
  thickness      cpu     quorra      cpu     quorra    its own area
       0.05    0        0.0510     0.0510   0.0510         0.05
       0.10    0        0.1020     0.1020   0.1020         0.10
       0.20    0.2471   0.2000     0.2000   0.2000         0.20
       0.50    0.4980   0.5020     0.5020   0.5020         0.50
       1.00    1.0000   1.0000     1.0000   1.0000         1.00

a 0.1-unit stroke at five placements    before            after
  where               cpu     quorra      cpu     quorra
  the top edge      0.0549   0.0980     0.0980   0.0980
  y 300             0.0941   0.1020     0.1020   0.1020
  y 160             0.0941   0.1020     0.1020   0.1020
  y 20              0.0941   0.1020     0.1020   0.1020
  the bottom edge   0.0549   0.0980     0.1020   0.0980
```

Every entry is within **one level of 255** of the shape's own area, which is where an eight-bit
raster runs out of places to put it: 0.05 of a row is 12.75 of 255 and the two backends round it to
13 and 13; the last row's 0.1020 against 0.0980 is 26 against 25.

### And what it does to shapes that are *not* degenerate, which was the constraint

`doc/todo/11` set the test: a rule that promoted every sub-quantum mark to a full mark would fight
the anti-aliasing departure on ordinary thin shapes. So the ladder was taken **across** the
one-pixel boundary where the substitution stops, error against the shape's own area:

```text
  thickness    cpu before   cpu after   quorra
       0.60      −16.99%      −0.00%    −0.00%
       0.80       −6.37%      −0.00%    −0.00%
       0.90      +11.11%      −0.22%    −0.22%
       0.95       +5.26%      −0.10%    −0.10%
       0.99       +1.01%      −0.18%    −0.18%
       1.00        0.00%       0.00%     0.00%
       1.01       −0.99%      −0.99%    +0.17%
       1.05       −4.76%      −4.76%    +0.09%
       1.20       +3.92%      +3.92%    −0.00%
```

Two results, and the second is the more interesting:

- **No step at the boundary.** 0.99 → 1.00 → 1.01 reads 0.9882, 1.0000, 1.0000; above one pixel the
  rasteriser's own quantum is untouched, and its ±5% there is the same number it always was.
- **The rule takes a promotion *away*.** At 0.9 of a pixel `tiny-skia` crossed all four sample
  lines and rounded to a whole row — 11% more ink than the document asked for — and it now draws
  0.898. The thing `doc/todo/11` was afraid this rule would do is the thing it undoes.

`issue8125.pdf` page 1 is that case on a real document and is why it left the oracle's contradicted
list: the page states one rectangle whose device extent is **0.882 of a pixel**, drawn 13% heavy,
now drawn as 0.598 of one row plus 0.284 of the next.

## What it costs

`callgrind_rasterise`, twenty renders, instructions:

```text
                                        before             after
ISO 32000-2 page 101              5,512,776,338     5,519,334,899    +0.12%
22060_A1_01_Plans.pdf page 1     84,473,016,854    84,474,994,548    +0.0023%
```

The gate is arranged so that the cheap questions come first — the width, the dash, the blend mode,
the transform, and `Path::narrowest_rectangle`, which is memoised on the path the way
`Path::collapses` is, so a fill asked once per strip walks its commands once per document. Only a
path that has passed all of those is stroked into an outline, which is the one allocation the rule
adds.

The plans drawing is the corpus's largest page of sub-pixel line work and it barely moves, which is
the same finding twice: **its rules are diagonals and polylines, which this rule declines**, so the
page pays only for the gate. That is a limit and it is written into `doc/todo/11` rather than
glossed.

## What the gates said

Every one of them, before and after, because this changes the oracle.

- **corpus** 974 documents, **70** incomplete, the same set.
- **oracle** 1794 pages, 1688 complete: agreeing **858 → 859**, contradicted **68 → 67**, ambiguous
  751, geometry 0/2, not comparable 9. **401 of 1794 pages' numbers moved**, which is what a
  rasteriser change looks like. One ratchet moved and is re-ratcheted with the argument above:
  `issue8125.pdf page 1` left `CONTRADICTED_SUBSTITUTED_FONT`, 19 names to 18.
- **quorra vs the CPU oracle** 957 pages, **914 agree / 42 differ → 920 / 36**, 1 refused and 17 not
  comparable unmoved. Six documents left, three from each list, and every one of them is the two
  rasterisers ceasing to disagree about a thin rectangle: `bug1308536`, `issue11913`, `issue13447`,
  `160F-2019`, `issue7454`, `issue840`. `issue16038.pdf` stays and moves **6.5359 → 1.8563**,
  similarity 0.90046 → 0.97723 — the largest movement that gate has recorded.
- **text** 99.2% (24 043 of 24 243), 25 below the floor — unmoved. **dates** 1514 of 1545. **XMP**
  318 read, 1 refused, 3191 properties. **JPEG 2000** 14 byte-identical.
- **workspace** 1323 → **1337** tests, 9 skipped. **conformance** 5119 → 5135 citations, 513 → 516
  quotations, 875 ledger rows and every status unmoved.
- **`doc/todo/00`'s step 7**, our ink minus the lightest reference's over all 786 ambiguous pages:
  head unchanged in order and every entry within 0.12, `issue16038.pdf` −5.642 → −5.758 and
  `issue4260_reduced.pdf` +17.577 → +17.607.

## What is left

- **A diagonal sub-pixel rule.** `22060_A1_01_Plans.pdf` is the whole corpus's worst case for it
  and is untouched. The substitution needs a pixel *line* to stretch into and a slanted band's
  pixel run is a staircase; what would answer it is a per-scanline coverage span rather than a
  rectangle, which is a different construction and not this one.
- **A thin shape that is not a rectangle** — a sliver of a triangle, a glyph stem. Declined
  deliberately, argued above, and no corpus page reports it.
- **The four-scale interior-coverage table in `AMBIGUOUS_TILING_CELL_CLIP`** has a 1× column
  measured before this change. It is marked as such rather than guessed at.
