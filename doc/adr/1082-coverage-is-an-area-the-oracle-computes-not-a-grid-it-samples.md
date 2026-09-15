# ADR 1082 — Coverage is an area the oracle computes, not a grid it samples

Status: accepted, 2026-09-15. Session 1068. Gives a **general** path the coverage §10.7.4's own
definition of a pixel implies, on the backend `CLAUDE.md` principle 2 makes the correctness oracle,
where `tiny-skia`'s supersampled path converter stated it on a lattice of sixteenths. Closes the
half of `doc/todo/_scan-conversion.md`'s departure (1) that ADR 0476, ADR 0583 and ADR 0226 left —
"every shape that is not axis-aligned rectangles", which is what a glyph, a curve, a diagonal and a
stroke's outline are made of. Takes 17 pages off the oracle's contradicted list and 6 off
`render-raster`'s differing list.

## What was owed

§10.7.4's first paragraph says where a shape's coordinates are *not* put:

> Its coordinates are mapped into device space but not rounded to device pixel boundaries.

and its third sentence says which way any remaining error may go:

> The area covered by painted pixels shall always be at least as large as the area of the original
> shape.

`tiny-skia` supersamples four times per pixel row and quantises a run along `x` to quarter-pixel
steps (`SUPERSAMPLE_SHIFT = 2`), so the only coverages it can state for a pixel one edge crosses
are the sixteen multiples of a sixteenth — reached by *rounding*, which means down as often as up,
and to nothing at all below a thirty-second. ADR 0476 took the axis-aligned rectangle off that
lattice, ADR 0583 a path stating several, ADR 0226 a shape thinner than a pixel. What was left was
everything else.

**Session 1064 measured it rather than asserting it**, which is why this round had a defect to fix
rather than a hypothesis. `examples/ink_ladder` showed eleven of `render-raster`'s twenty-one
differing pages agreeing on *how much* ink there is at every rung of 1×, 2×, 4× and 8×, so what
separated the backends on those pages was placement alone; a 1/16-lattice count then said whose
placement was the coarser. **This round re-took that count with its control** — the instrument is
`render-raster/examples/coverage_lattice.rs`, and the control is the second backend, which resolves
a path analytically and must therefore read near chance:

| page | cpu before | cpu after | raster (control) |
|---|---|---|---|
| `endchar` | 100.0% | 14.2% | 15.0% |
| `copy_paste_ligatures` | 100.0% | 14.9% | 21.7% |
| `bug1743245` | 99.8% | 81.0% | 80.4% |
| `issue2884_reduced` | 99.8% | 21.8% | 24.5% |
| `issue16473` | 78.0% | 6.7% | 7.1% |
| `bug1844583` | 77.7% | 8.3% | 7.1% |
| `issue20232` | 74.0% | 19.1% | 15.0% |
| `pr12564` | 72.7% | 16.3% | 17.5% |

Chance, for the tolerance band the instrument uses, is 25.0%. Two pages read high on **both**
backends after the change (`bug1743245` 81.0 against 80.4, `issue19083` 71.8 against 70.1), which
is the control saying those pages' levels cluster for reasons of their own — a count that came back
high on one backend only is what the defect looked like.

The **counts** move too, and they say the same thing from the other side: the partial pixels
`endchar` states go 86 → 120 against raster's 120, `issue16316` 260 → 288 against 288,
`bug1844583` 894 → 941 against 940. The oracle was not merely rounding a boundary pixel's coverage;
below a thirty-second of a pixel it was rounding it away.

## The construction, and it is derived rather than adapted

§10.7.4 identifies a pixel by flooring a point and gives it `[i, i+1) × [j, j+1)`; §8.5.3.3 defines
insideness by a **winding number**. So a path's coverage of a pixel is the integral of its winding
number over that square, read through whichever fill rule is in force. `render_cpu::area` computes
that integral. For one directed edge crossing one pixel row, the part of column `i` lying to the
right of the edge at height `y` is `clamp(i + 1 − X(y), 0, 1)`, so the edge's share of the integral
is

```text
  S(i) = w · ∫ clamp(i + 1 − X(y), 0, 1) dy
```

`S` is zero left of the edge and `w · dy` right of it, so its **differences** are supported only on
the columns the edge touches: accumulate the differences of every edge, prefix-sum each row, and
the sum at column `i` is the whole path's winding integral there. Over one row the edge is a
straight segment, so substituting `w = u − x` turns the integral into a difference of
antiderivatives of a clamped ramp — two evaluations, exact at every placement including a vertical
edge. The non-zero rule takes the magnitude of the sum capped at one; the even-odd rule folds it
into `0..=1` with period two.

This is the accumulating scan converter every font rasteriser has, written from the clause instead
of transliterated: the module states the integral, the antiderivative and the differencing, and the
branches are the three ways a crossing can meet the region's own columns.

## The one path it declines, and §11.6.2 is what says so

The sum is the integral of the **winding number**, which is the filled set's own indicator only
while that number stays inside `-1..=1`. Where two portions of one path wound the same way cover one
region, the number there is two, and a pixel on the *boundary* of that region reads twice the area
it covers — which is portions of an object composited with one another:

> Portions of an object shall not be composited with one another, even if they are described in a
> way that would seem to cause overlaps (such as a self-intersecting path, combined fill and stroke
> of a path, or a shading pattern containing an overlap or fold-over).

`tiny-skia`'s converter applies the fill rule to each sample, so it has never had this and is the
right answer for such a path; it measures the result to a sixteenth, which is the trade. **The
condition is the arithmetic's own**: a cell holds the change in the winding integral across one
pixel, and one edge can change it by at most one whole winding — its contribution is a share of the
row's height times a fraction of the column, both inside `0..=1`. So a cell past one is a pixel two
same-wound edges *cross*, the mark goes to the library's converter, and nothing without such a
crossing is ever declined.

**The condition was `|Σ| > 1` on the accumulated sum for one round of the round, and that was too
blunt to keep** — it declined every path with an overlap anywhere, including the one that is
harmless. A glyph whose stems overlap has winding two *inside* it, where the clamp gives one and is
right; only a crossing *on the outline* is wrong. Measured on `standard_fonts.pdf` page 1, the
accumulated test declined **514 of 854 marks** and cost the page **+28.6%**, because a decline pays
for the accumulation and then for the library's conversion as well; the per-cell test declines
almost none of them and the page costs **+1.2%**. What the narrower test does not catch is a
crossing where the covered fraction is under a half, and the error there is that fraction — under
half a pixel, on the heavy side the `shall` permits.

`pdf-model/tests/glyph_clip_direction.rs` is the scene that watches it, and it is §9.3.6's: text
rendering mode 7 accumulates a glyph's outline into the clipping path, so a word set twice in one
place is one path stating every outline twice.

**Two constants are chosen and both are measured.** The flattening tolerance is 1/256 of a device
pixel, derived from the raster's own depth — a chord `d` pixels from its curve moves a boundary
pixel's coverage by at most `d` — and calibrated on the corpus's most curve-dense page: at 1/16,
1/64, 1/256 and 1/1024 `issue2177.pdf` reads 13002.05, 13022.88, **13030.50** and 13031.90 of ink,
so the chosen value is within **0.011%** of its own limit and twice the segments buys nothing. The
cell budget is a cost guard and not a condition; a mark past it is drawn the way every mark was
drawn before.

**And one rule is deliberately *not* carried over.** `pdf_render::expressible_coverage` states a
positive coverage under one level *at* one level (ADR 0419), and every other exact construction in
this backend applies it. This one does not, and the difference is what the coverage is made of: that
function is for a mark whose area was moved into the paint's alpha by a substitution, where a
coverage rounding to nothing loses the mark the substitution was built to keep. Here the coverage is
the shape's own area where it lies, a shape too small for the raster is `collapsed`'s,
`sub_pixel_bands`' and `point_mark`'s — each of which runs *before* this converter — and rounding to
nearest keeps the accumulator's own residue, of order `2⁻²⁴` per cell, an order of magnitude below
the first level rather than lifted on to it.

## The two-contour outline, which is the same clause one operator over

`issue15150.pdf`'s whole content stream is `0.5 w 1 0 0 RG 0 9.75 m 0.5 9.75 l s`, and its stroked
region is the device rectangle `[0, 0.5] × [0, 0.5]` — a quarter of pixel (0, 0). The oracle drew
**0.1875** of that pixel. `s` closes the subpath, so `tiny-skia`'s stroker returns the outline as
**two contours of the same rectangle, wound the same way**, the inner one carrying a vertex part way
along each vertical side; `pdf_render::is_axis_aligned_rectangle` required a subpath's four
`LineTo`s to *be* the four corners, so it declined, `sub_pixel_bands` declined with it, and the rule
fell to ADR 0268's widened band whose ink ran off the top of the raster.

Two things moved and both are stated from clauses rather than from the shape of the stroker's
output. **A side may be stated in more than one segment**: a vertex lying part way along a side adds
no corner and changes no area, so the walk records a corner where the axis a side runs along
*changes*, and a side may be any number of segments long. **And a rectangle stated twice the same
way round is one region**: §8.5.3.3's non-zero rule counts windings, so two coincident contours of
equal orientation name the same set of points as one, which is one portion of the object rather than
two competing for a device pixel line — `contested` declined the pair. The predicate returns the
traversal's orientation for that reason, because a bounding box cannot say it and two coincident
contours of *opposite* orientation name no points at all, which no substitute here can express and
which is therefore declined to the rasteriser.

The page now draws **0.251** of pixel (0, 0) — the area itself. It stays on `render-raster`'s
differing list, and the backend it convicts has changed sides: raster draws 0.5, twice the area, at
this scale only.

## What it cost

`callgrind_rasterise`, five rasterisations, `RAYON_NUM_THREADS=1`, both arms from one binary so that
trap 16 has nothing to say about which program each figure came from:

| page | before | after | |
|---|---|---|---|
| `standard_fonts.pdf` p1, type | 681,907,221 | 690,241,690 | **+1.22%** |
| ISO 32000-2 p101, text | 1,378,039,240 | 1,426,869,683 | **+3.54%** |
| `issue13447.pdf` p1 | 1,606,194,105 | 1,828,789,069 | **+13.86%** |
| `issue840.pdf` p1 | 1,513,061,734 | 1,763,402,732 | **+16.54%** |

Type and text are near free; the two that pay are vector pages with many transparency groups.
**Two thirds of what they would have paid was found by measuring rather than by assuming, and both
findings are in the code beside the fix.** The first cost 12% of the text page: `f32::floor` and
`f32::ceil` are calls into the platform's maths library on any target without SSE 4.1 and this
converter asks for them three times per edge per row, so the rounding is a cast, which *is* the
floor for a value at or above zero. The second cost 22% of `issue840.pdf`: `scan::intersected`'s
coverage buffer was one `Mask`, which carries its own dimensions, and every transparency group draws
into a pixmap of its own extent — so the buffer was discarded and reallocated **361 times per
rasterisation**, and `Mask::new` zeroes. Keeping the bytes and reshaping them moved the same cost
into `Vec::resize`; keeping one buffer per extent removes it.

**Both of this backend's jobs are measured and only one of them is on a clock.** `CLAUDE.md`
principle 2 puts page one on the graphics device, so `render-cpu` is the oracle and the fallback and
is not the shipped path; `pdf-transform --test gate` is nevertheless the floor it owes, and it reads
**186.9 to 201.9 pages/s over three runs, against a floor of 40**, with the converter in. That spread
is not a comparison with anything — a wall clock on a machine carrying five other rounds measures the neighbours, which is
this tree's standing habit — and the instruction counts above are.

## What moved

- **The oracle: 993 agrees / 62 contradicted → 1010 / 45**, ambiguous unchanged at 835. All
  seventeen came out of `CONTRADICTED_GLYPH_EDGES`, whose diagnosis was that the marks are the right
  marks and the letters' *edges* are the difference — proved by conserved ink, by an identical
  bounding box at 8× and by a page that draws its glyphs twice. The group named a departure this
  tree could close, and closing it emptied two thirds of it.
- **`render-raster`'s differing list: 930 agree / 21 differ → 935 / 16.** Six left —
  `bug1743245`, `bug1978317`, `copy_paste_ligatures`, `endchar`, `issue16316` and
  `issue2884_reduced`, five of them the pages 1064 read as the quantum; `issue21068.pdf` left with
  them and came back when the decline above was added, its `1 w` comb separators being strokes whose
  outlines cross themselves. `issue2177.pdf` arrived, and
  it is a second analytic answer rather than a quantum: a page of small ellipses where almost every
  inked pixel is somebody's curve boundary, the two backends 0.73% apart at 1× and 0.17% at 8×, and
  both heavier at the page's own scale than at eight times it — which is the side the third sentence
  asks for.
- **`raster_golden`: 719 of its 974 lines moved**, and **every mover is `raster only`** — the
  display list and the reports are byte-identical on all of them, which is trap 37's test that a
  rasteriser change changed the rasteriser and nothing else. The ink distribution is
  `pdf-model/examples/own_ink.rs`, new here: of the 702 whose ink moved at all, **456 up and 246
  down**, median **+0.00158** levels of 255 of mean coverage, quartiles −0.0010 and +0.0209, 385
  under a hundredth of a level and only 21 of 974 as much as a fifth. Nothing reaches a whole level.
  The tail is `issue16316.pdf` −0.99, a line of type whose stems were being rounded *up* to a
  quarter, and the head `copy_paste_ligatures.pdf` +0.50.

## What was declined

- **Raising `tiny-skia`'s supersampling.** `SUPERSAMPLE_SHIFT` is a private constant of a
  dependency, so this is a fork; and it would buy a finer lattice where the clause asks for no
  lattice. It also halves the coordinate range the library's fixed point can express, which
  `scan::SUPERSAMPLED_LIMIT` is already the boundary of.
- **The rectangle path.** A single axis-aligned rectangle keeps ADR 0476's closed form and
  `tiny-skia`'s rectangle scan converter, which is exact *and* cheaper; this module is asked only
  where `pdf_render::edge` says the mark is not rectangles.
- **A tighter flattening tolerance**, on the calibration above: 1/1024 moves the corpus's most
  curve-dense page by 0.011% for four times the chords.
- **A finer condition than "a cell past one whole winding".** What is left uncaught is a crossing
  where the covered fraction is under a half, and separating that from a single edge needs the
  filled set rather than its winding — a conflation-free converter, which is a different
  construction and not this one.
- **The surface-sized coverage buffer.** The construction delivers its coverage as a mask, and
  `tiny-skia` takes a mask only at the pixmap's own size, so a mark reaching two rows still costs a
  buffer as tall as the surface. Two buffers per extent removes the reallocation; what remains is
  one clear and one blit over the mark's own reach, which is where `issue840.pdf`'s remaining +10%
  lives. A narrower construction would need this backend's own blitter, which is the step ADR 0355
  priced and declined for the same reason.
