# ADR 0154 — A fill with no area may not disappear

Status: accepted, 2026-08-03. Session 186. The defect the hundred-and-eighty-fourth session
recorded in `AMBIGUOUS_ZERO_AREA_FILL` and did not fix.

## The page

`issue4260_reduced.pdf` rules a grid, and every line of it is a rectangle with a zero side:

```text
848 1085 10159 0 re f
848 1281 10159 0 re f
```

Three references draw the grid. We drew the surrounding box and nothing inside it, because a
shape with no area has no coverage and an antialiasing rasteriser paints in proportion to
coverage. The page sat at 13 bounds from the nearest reference inside an `ambiguous` verdict,
which is §3a's whole argument: the verdict means "nobody agrees closely enough to call anybody
wrong", and it cannot tell a grid from a blank.

## The reading, which is not the easy one

§10.7.4 looks like it settles the question in one sentence:

> A shape shall be scan-converted by painting any pixel whose half-open square region intersects
> the shape, no matter how small the intersection is. This ensures that no shape ever disappears
> as a result of unfavourable placement relative to the device pixel grid, as might happen with
> other possible scan conversion rules.

It does not, and the reason is two paragraphs above it in the same subclause:

> Like pixels, shapes to be painted by filling … and stroking … operations are also treated as
> half-open regions that include the boundaries along their "floor" sides, but not along their
> "ceiling" sides.

For a rectangle whose floor and ceiling are the *same* line, that rule read literally leaves the
region empty — `1085 ≤ y < 1085` is no point at all — and an empty region has nothing to paint.
So the subclause's two halves collide, and the second sentence's "as a result of unfavourable
placement" can be read as limiting its guarantee to shapes that have an area and land badly.
Neither half is subordinate to the other on its face.

**What settles it is the neighbouring clause.** §8.5.3.3.1:

> If a subpath is degenerate (consists entirely of one or more points at the same coordinates),
> the subpath shall be considered to enclose the single device pixel lying under that point

A single point is the *most* degenerate shape a fill can have, and the standard states outright
that it encloses a pixel. A zero-height rectangle contains such points, so a reading under which
it paints nothing makes the smaller shape mark more of the page than the larger one containing
it. The half-open convention exists so that two shapes sharing an edge neither overlap nor leave
a seam; using it to annihilate a shape is not the job it was given.

The same pair of clauses says why the *point* stays a documented departure while this does not.
§8.5.3.3.1 calls its own answer "device-dependent and not generally useful" in the same breath as
stating it; §10.7.4 attaches no such hedge, and a producer that writes a ten-thousand-unit rule
means a line rather than an accident.

## The rule

`pdf_render::collapsed::split_collapsed_fill` separates a fill path into the subpaths that
enclose an area and the marks left by those that do not. A subpath collapses when its extent
along exactly one axis is **zero** — compared exactly, not within a margin — and the mark is a
rectangle one device pixel thick, centred on the line the subpath lies along and as long as the
subpath is.

Four decisions, each with its reason:

- **The width is not a new one.** One device pixel expressed in the path's own space is what
  §8.4.3.2 gives a zero-width line and §10.7.5 a line under half a pixel; `Stroke::device_width`
  already asked for it. It is now `pdf_render::thinnest_line`, one function with three callers,
  so that a device's minimum cannot come to have two values.
- **The mark is geometry in the shared crate**, not a hairline each backend applies, for the
  reason `degenerate.rs` states a circle rather than trusting a round cap: a decision either
  backend can make alone is a decision neither has made (trap 2). `render-cpu` and `render-gpu`
  each fill the same rectangles.
- **The marks are filled beside the path, under the non-zero rule**, rather than appended to it.
  A mark inside an even-odd path's own outline would toggle parity and punch a hole in what it was
  meant to draw — checked by a test that fills a square with a flat subpath inside it.
- **Exactly zero, not sub-pixel.** A shape thinner than a pixel but not *flat* has an area, and
  what this renderer does with it is the antialiasing departure `CONTRADICTED_ANTIALIASED_EDGES`
  already argues. Only a subpath with no extent vanishes at every placement and every resolution,
  which makes this rule a statement about the geometry rather than about the scale — and that is
  what lets `Path::collapses` memoise the answer once for a path drawn at every zoom.

## What it cost, measured

`Path::collapses` is a `OnceLock<bool>` beside the hull `Path` has carried since session 163, and
for the same reason: the question is asked once per fill command per strip, which was 17.6% of a
dense page when `Path::bounds` walked its points every time. Rasterisation of the specification's
own pages, by `callgrind_rasterise` against session 185's figures: page 6 **4 041.7 M** against
4 023.7 M (+0.45%), page 101 **5 551.3 M** against 5 566.0 M (−0.26%) — both inside the repeat
noise of the instrument. Interpretation is untouched; nothing on that path asks.

## What it bought

- `issue4260_reduced.pdf` draws its grid. Ink over the page, `255 − mean` on the oracle's own
  artefacts: ours **19.79**, `hayro` **19.83**, `ghostscript` 6.29, `poppler` 3.51, `mupdf` 2.16.
  The two Rust renderers put down the full mark the clause asks for; the three C ones shade it by
  under a fifth, which is the antialiasing departure seen from the other side.
- Every oracle verdict, corpus count and text percentage is unchanged: 847 agreeing, 70
  contradicted, 751 ambiguous, 79 incomplete. The page keeps its `ambiguous` verdict for the
  honest reason — the references disagree with one another about the weight — and stays in
  `AMBIGUOUS_ZERO_AREA_FILL` with the measurement above written into it.

## What it found and did not fix

Writing the pixel test measured the boundary, and the boundary is not where the geometry says.
`tiny-skia` samples four times per row and rounds, so a filled sliver **with** an area also
disappears: on an 80-unit rule at scale 1.0 the ink against the area's own answer is 0.05 units →
0, 0.1 → 0, 0.2 → 19.8 of 16, 0.5 → 39.8 of 40. That is §10.7.4's sentence again one step along,
and it is a different rule — a fact about the *device's* coverage quantum rather than about the
shape — so it is recorded in the ledger and in the handover rather than folded into this one. The
GPU backend's own quantum has not been measured.
