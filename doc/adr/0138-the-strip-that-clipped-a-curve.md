# ADR 0138 — The strip that clipped a curve

Status: accepted, 2026-08-02. Session 154. **Built, measured, and not shipped.** ADR 0137 said the
decomposition does not forbid parallel CPU rasterisation and named the ceiling; this is what
happened when it was written, and the answer is four oracle pages.

## What was built

`CpuRasterizer::rasterize` cut the target into strips at the boundaries ADR 0137's planner chose,
gave each one a `TargetSpec` of its own height whose transform carried its first row as an offset,
borrowed disjoint runs of the pixmap's bytes so nothing was copied or shared, and ran them on
rayon. `encode`, `draw_group` and `build_soft_mask` moved from `&mut Pixmap` to
`&mut PixmapMut<'_>` so a strip could be a borrowed view; each strip got its own `MaskCache` with
the budget divided by the strip count, because the masks of a strip are a strip tall. A command
whose device bounds missed the target was dropped before its path was built, without which every
strip would have paid the 19% of per-command work for every command on the page.

It worked. It also changed the picture.

## What it cost

`crates/render-cpu/tests/strip_parallelism.rs` rendered each cross-backend scene serially and at
2, 3, 5, 8 and 16 strips and demanded the bytes be **equal**. Three scenes failed:

| scene | strips | bytes differing of 32 M | worst byte |
|---|---|---|---|
| `curves` (one Bézier fill) | 3 | 3982 | **64** |
| `diagonal_stroke` | 2 | 422 | 16 |
| `knockout_group` | 2 | 247 | 16 |

64 of 255 is a quarter of a channel, which is one of the four sub-scanlines a supersampling
anti-aliaser averages. That is not floating-point rounding of the row offset, and a probe said
what it was.

## The mechanism, isolated

Filling one path — a single closed Bézier, no clip, no shading — into a 2382×3368 pixmap, and then
into a top piece and a bottom piece with the transform translated by the split:

| split at row | bytes differing above it | below it | worst |
|---|---|---|---|
| 400 | 0 | 0 | 0 |
| 1123 | 594 | 548 | 48 |
| 1684 | 1306 | 74 | 48 |
| 2000 | 324 | 1662 | 64 |

**Row 400 is exact and the rest are not, and the difference is which pieces the path crosses.** The
shape spans device rows 567 to 2967: at a split of 400 it lies wholly inside the lower piece and
neither piece clips it, so both are bit-identical to the whole. At every other split the path is
cut by a pixmap edge — and a curve clipped at an edge is re-parameterised, so the clipped curve is
not the same sequence of `f32` control points the unclipped one was. The coverage it produces
differs in the last part of an edge pixel.

**This is not a defect in `tiny-skia` and not one in the strip driver.** It is what clipping a
curve costs, and any decomposition that cuts through ink pays it.

## Why that is disqualifying rather than a tolerance to write down

The natural reply is that a quarter of a level on a thousand of 32 million bytes cannot matter.
The oracle disagrees, and it is the instrument this project keeps for exactly this question:

```
4 page(s) newly contradicted by the reference consensus:
  bug1811694.pdf page 1, dates.pdf page 1, issue14705.pdf page 1, issue15597.pdf page 1
```

Four pages that two independent implementations agree about, and we did, stopped agreeing. Run
again with the strip count forced to one and everything else in place — the same skip test, the
same planner, the same refactored surfaces — the oracle is clean at its recorded 836 agreeing
pages. **So the skip test is right and the strips are what cost the four pages**, which is a
measurement rather than an inference: the two runs differ in one variable.

Two further reasons the difference cannot be absorbed:

- **`render-cpu` is the oracle the GPU backend is judged against** (`lib.rs`'s own second
  sentence). A backend whose output depends on how it divided the page makes every cross-backend
  comparison inherit that noise, and those tolerances are already tuned to a tenth of a level in
  places (trap 12).
- **How many cores a machine has would decide what a document looks like.** That can be removed by
  fixing the strip count instead of reading `available_parallelism`, and it should be if this is
  ever revisited — but it does not address the four pages, which differ from the serial render at
  *any* strip count.

## What would make it work, and it is not a tolerance

The probe says exactly when a strip is exact: **when no path crosses its boundary.** So the split
would have to choose its cut rows from those no command's extent — and no clip chain's, and no soft
mask's — strictly straddles. ADR 0137 already measured that a command touches 1.01 to 1.13 strips
of eight, so most rows are candidates on a text page, and `strip_spans` already computes the
extents that would decide it.

It is not obviously achievable. Page 6 of ISO 32000-2 states one page-wide clip (ADR 0132), which
straddles every row; a page like that would have no legal cut and would stay serial. Whether an
axis-aligned rectangular clip cut by a horizontal edge is exact — plausible, since nothing is
re-parameterised — is a probe somebody could run in ten minutes, and it decides whether the common
case survives the constraint.

**That is the shape of the next attempt, and this ADR is not it.** What is kept from this one:

- `Path::bounds` and `Command::device_bounds` in `pdf-render`, and `pdf_render::strips`, which is
  the planner ADR 0137 measured — now the shipped code the `strip_spans` example calls, rather than
  a second copy in an example that could drift from it.
- The knowledge that the skip test is exact over 1794 oracle pages, which is worth having if a
  later attempt needs it.

What is reverted: every line of the parallel driver, the `PixmapMut` refactor, and `render-cpu`'s
dependency on rayon. `CLAUDE.md` forbids shipping a path nobody takes, and a parallel renderer
switched off is exactly that.

## The habit this is the second instance of

ADR 0131 priced a glyph coverage cache, found it needed a positional departure, applied the
departure, asked the oracle, and refused it. This is the same shape one level up: **an optimisation
whose correctness cost is invisible in a unit test and visible in the oracle.** In both cases the
cheap step — write it, then ask the instrument — was worth more than any amount of reasoning about
whether the difference could matter.

And one thing that is new. The equality test failed on three scenes out of eight, and the three
were the ones with a *curve* crossing a boundary. **A suite of shapes is a suite of shapes**: had
`test-scenes` held only rectangles, this would have passed every test in the tree and been found by
the oracle four pages later — or not at all, since a corpus page nobody looks at can differ inside
an `ambiguous` verdict.
