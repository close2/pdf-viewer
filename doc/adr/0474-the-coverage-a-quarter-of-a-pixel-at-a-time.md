# ADR 0474 — The coverage this backend paints, a quarter of a pixel at a time

Status: accepted, 2026-08-21. Session 643. Empties `CONTRADICTED_ANTIALIASED_EDGES` and moves
`colors.pdf` pages 1 and 2 to `CONTRADICTED_TIGHT_CONSENSUS` with the page's own closed form beside
them; amends §10.7.4's ledger row and `doc/todo/_scan-conversion.md`'s departure (1); opens
`doc/todo/11` item 7 with a price; adds two instruments,
`render-quorra/examples/edge_coverage_ladder` and `pdf-model/examples/compare_rasters`. **No pixel
moves.**

## How the group was chosen, and the tell was again in its own note

Trap 1 in `doc/traps/pixels-and-rasterisers.md` says a contradicted page's group names a hypothesis
rather than a diagnosis — eleven for eleven when this round started — and that a group whose note
argues *another* group's mechanism is the cheapest tell there is.
`CONTRADICTED_ANTIALIASED_EDGES` held two pages and its note ended:

> The bound is what makes it fail at all, and it is trap 12's shape: mean 0.25 against 1.00 and
> worst tile 2.79 against 5.00 both pass with room, and structural similarity fails at 0.9857
> against **0.9886** — a bound the two least-anti-aliased renderers set for each other on a page
> that is nothing but edges.

That is `CONTRADICTED_TIGHT_CONSENSUS`'s sentence, under a name asserting a cause — *our*
anti-aliasing, softer than anybody's — that no line under it measured. The same shape as session
583's `CONTRADICTED_MASK_QUANTISATION`, one group over.

## What the pages are

`colors.pdf` pages 1 and 2 are 595 × 841 points, rendered one pixel per point, and each holds
**sixteen axis-aligned rectangles and nothing else**: a white ground and a 3 × 5 grid of flat
`rg` swatches, each stated as `m l l l h f*` under `1 0 0 -1 0 841 cm` and
`0.001968504 0 0 0.001968504 0 0 cm`. `0.001968504` is `1/508` to a part in thirty million, so every
boundary in the file is a known fraction of a device pixel — the column boundaries are `100800/508`
and `201600/508`, device x **198.4252** and **396.8504** — and no glyph, curve, image or clip is on
either page.

That makes the page a **closed form** rather than a comparison. A rectangle's coverage of a pixel is
the product of its two one-dimensional overlaps; compositing the sixteen in the order the content
stream states them, source-over, gives every pixel of the page out of the file's own arithmetic with
no renderer in it.

## The measurement

Two forms were written out, pixel by pixel, and rendered to PNG: one using the **exact** overlap,
one using the overlap `tiny-skia` measures — its scan converter supersamples four times per axis,
at 0.125, 0.375, 0.625 and 0.875, so a quarter of a pixel is its quantum. Each was then compared
with the oracle's own artefacts by the oracle's own arithmetic
(`pdf-model/examples/compare_rasters`, which is `raster_compare::compare` pointed at two files):

| page 1 | vs the exact form | vs the quarter-quantised form |
|---|---|---|
| ours | mean 0.0406, max **33** | mean 0.0023, max **1**, ssim 1.00000 |
| `hayro` | mean 0.0015, max **2**, ssim 1.00000 | mean 0.0545, max 33 |
| `mupdf` | mean 0.0173, max 13 | mean 0.0526, max 25 |
| `ghostscript` | mean 0.1368, max 54 | mean 0.1791, max 64 |
| `poppler` | mean 0.2823, max 124 | mean 0.3313, max 130 |

Page 2 reproduces it: ours to the quantised form mean 0.0022 and max 1, `hayro` to the exact form
mean 0.0017 and max 2.

**Our raster *is* the quantised closed form, to one level of 255 over half a million pixels, and
`hayro`'s is the exact one.** So the note's picture — five renderers on a spectrum of softness with
us at the soft end — is wrong twice over. Ranked by distance from the page's own geometry at the
worst pixel the order is `hayro` **2**, `mupdf` **13**, ours **33**, `ghostscript` **54**, `poppler`
**124**: two of the five paint the area the shape covers, `poppler` paints whole pixels,
`ghostscript` supersamples and filters, and
ours is the geometry with each edge's coverage **rounded to a quarter**, up in most places, down in
others, and to nothing wherever an edge covers less than an eighth of its pixel. **We are third of
five and not the outlier**, and the difference is not softness but coarseness, in both directions.

## The instrument, with no document in the way, and it names the backend

`render-quorra/examples/edge_coverage_ladder` fills one rectangle whose edge is placed every
twentieth of a pixel and reads the boundary pixel, on both backends, against the fraction the shape
covers:

```text
  shape   0.05  0.10  0.15  0.25  0.35  0.40  0.60  0.65  0.85  0.90
  cpu     0.00  0.00  0.25  0.25  0.25  0.50  0.50  0.75  0.75  1.00
  quorra  0.05  0.10  0.15  0.25  0.35  0.40  0.60  0.65  0.85  0.90
```

Both axes read alike, to a level of 255 on the graphics device and to a quarter on the processor.
The vertical edge is a partial run inside a row and the horizontal one is a row partly inside the
shape, and `tiny-skia` measures both by counting sub-samples: an *axis-aligned* edge is seen the
same way by all four sub-rows, so the sixteenth of a pixel that is this converter's quantum for a
general shape becomes a **quarter** for the commonest shape in every PDF.

**And `poppler` is not the clause either**, which the same ladder says and is worth recording
because ADR 0308's version of this sentence stops one step earlier. Restated as twenty-one
one-rectangle PDFs — the same ladder written out as files — and put to `pdftoppm -cropbox -r 72`,
`poppler` reads 0.000 up to a shape fraction of 0.45 and 1.000 from 0.50 on: it snaps an
axis-aligned edge to the *nearest* pixel boundary, where §10.7.4 rounds **outward**, "painting any
pixel whose half-open square region intersects the shape, no matter how small the intersection is".
So the renderer the old note called "closest to the clause" drops an edge covering nine tenths of a
pixel, and every renderer on this page departs from the subclause in one direction or the other.
`mutool draw -r 72` tracks the fraction throughout — 0.059, 0.118, 0.176 for the first three rungs —
which is what puts it 13 levels from the geometry on the page rather than 124.

**The quantum is `render-cpu`'s alone**, which is the same shape ADR 0226 found for the two
disappearances — the device drew those correctly too — and it is the awkward half: the CPU
backend is this project's correctness oracle, and on an axis-aligned edge it is the less exact of
the two rasterisers it arbitrates between.

## Whose defect, and what the clause says

§10.7.4, verbatim:

> A shape shall be scan-converted by painting any pixel whose half-open square region intersects
> the shape, no matter how small the intersection is. This ensures that no shape ever disappears as
> a result of unfavourable placement relative to the device pixel grid, as might happen with other
> possible scan conversion rules. The area covered by painted pixels shall always be at least as
> large as the area of the original shape.

`tools/spec-errata emit` on `ISO_32000-2_sponsored_EC3.pdf` carries no annotation against §10.7.4 —
the only erratum in the §10.7 family is #371 on §10.7.2's flatness — so those sentences stand as
printed.

Anti-aliasing is a departure from the first sentence, licensed by §10.7.1's NOTE that the algorithm
"is not defined by PDF", and `doc/todo/_scan-conversion.md` has recorded it since the sixteenth
session. What the licence does not cover is the third sentence in the *rounding-down* direction. An
edge covering a tenth of its pixel paints **nothing** here, so the painted area is smaller than the
shape's — which is the failure ADR 0226 and ADR 0268 were written to pay for a shape thinner than a
pixel, met again at the edge of one thicker than a pixel. This file's own rule for telling the
departure from a defect, written in `_scan-conversion.md` under `AMBIGUOUS_TILING_CELL_CLIP`, is
"anti-aliasing gives the shape's area; coming out *under* it is a defect". By that rule this is a
defect, and it is recorded as one.

## Why the pages still moved to a bound group, and why nothing was changed in the rasteriser

Because the exact form is contradicted too, which is the one thing the diagnosis could not assume.
The gate's bound is twice the consensus pair's own distance, and here the pair is `poppler` and
`ghostscript` — the two renderers furthest from the geometry:

```text
                        page 1     page 2
  bound                0.98862    0.98402
  ours                 0.98591    0.97906
  the exact form       0.98772    0.98001
```

**A rasteriser painting precisely the area each rectangle covers would be contradicted on both
pages.** So the verdict is trap 12's — a bound tighter than the arithmetic — and the pages belong
with `issue7891_bc1.pdf`, with the closed form beside them as that group's note asks. Fixing the
quantum would move both pages *toward* the bound and past neither.

That is the whole reason no code changed. The cure is not hard to describe — an axis-aligned
rectangle's coverage is a product of two overlaps, so the interior at full alpha plus up to eight
edge and corner pieces at their own exact coverage draws it exactly, in the shape `sub_pixel_bands`
already uses and under the same `carries_coverage_as_alpha` condition — and it is not cheap to take:
nine `scan::fill` calls where there was one, on the commonest command in the corpus, in a project
whose second principle is latency; and every axis-aligned edge on every gated page moving by up to
an eighth of its coverage, which is trap 1's sweep over a population no change in this tree has
touched. A round that takes it has to measure both ends. `doc/todo/11` item 7 carries it with those
two costs written down, in ADR 0308's shape: record the artefact with its price rather than chase
it.

## What was checked and was not the answer

- **Not the seam.** Ours does leave the ground showing where two swatches abut — 63 of 255 in the
  blue channel at one sampled boundary, where both swatches state blue 0 — but so does every
  anti-aliasing renderer here, `hayro` and `mupdf` included, and ADR 0308 settled that it is
  §11.3.7.3's union applied to fractional shapes. The closed forms above **include** the seam, in
  both variants, which is why they reproduce every renderer's raster and not only the differences.
- **Not `sub_pixel_bands`.** It applies to a rectangle thinner than a device pixel; these are 198
  pixels across.
- **Not the low-precision pipeline** ADR 0418 turned off. Ours reproduces the quantised form to one
  level, which is what an exact composition of quantised coverages looks like.
- **Not a page-size or rounding difference.** All five rasters are 595 × 841 and every swatch
  interior is identical in all of them.
