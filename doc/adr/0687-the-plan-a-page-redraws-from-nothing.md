# ADR 0687 — The plan a divided page pays for before it draws anything

Status: accepted, 2026-08-25. Session 762. `pdf_render::unsplittable_rows` stops walking a path
whose every reachable row is already forbidden: page 101 of ISO 32000-2 rasterises **7.08%** cheaper,
byte-identically, and the *serial* prologue in front of the divided render falls **4.8×**.
Clauses: ISO 32000-2 §10.7.4, whose row carries the strip planner's exactness argument and did not
name its code.

## How this was found

The general-improvement rule these rounds have converged on is: find a number this project wrote
down and has not re-run, and prefer a **composition** to a total, because a total is what a later
round re-takes and a breakdown is not.

`callgrind_rasterise`'s total for page 101 has been re-taken repeatedly — sessions 153, 162, 163,
175, 185, 195, and four rounds ago in ADR 0677's own table. Its **composition** was last taken in
the hundred-and-fifty-third session and refined once in the hundred-and-sixty-third. That
composition is in `doc/performance.md` and says:

> of which `CpuRasterizer::draw` is 4 104 M, and inside it `render_cpu::convert::path` 405 M,
> `Rect::from_points` 218 M and `RasterPipelineBlitter::new` 164 M are **787 M — 19% of the
> render — of per-command work that does not shrink with the band**

Six hundred rounds later two of those three are gone — `convert::path` is 45 M and
`RasterPipelineBlitter::new` 24.5 M — and the third is where it was. What had taken their place was
a function no document in this tree had ever named:

| page 101 ×20, before | Ir | share |
|---|---:|---:|
| `tiny_skia::scan::path::fill_path_impl` | 785.7 M | 14.52% |
| `tiny_skia::edge_builder::BasicEdgeBuilder::push_cubic` | 465.6 M | 8.60% |
| **`pdf_render::strips::segments`** | **373.7 M** | **6.90%** |
| `tiny_skia::pipeline::highp::load_dst_tail` | 270.1 M | 4.99% |
| `SuperBlitter::blit_h` | 255.2 M | 4.72% |
| `callgrind_rasterise::main` — the example's own ink sum | 254.7 M | 4.71% |
| `quicksort::<tiny_skia::edge::Edge>` | 231.8 M | 4.28% |
| `tiny_skia_path::Rect::from_points` | 227.9 M | 4.21% |

The third row is ours, and the caller tree says what it belongs to:
`CpuRasterizer::encode_in_strips` calls `strips::segments` **twenty times** — once per render — for
**448.3 M inclusive, 8.28% of the page**, with `strips::gather` beside it for another 24.7 M. The
whole planning prologue is **494.0 M, 9.13%**.

## Why 9% of a total understates it

`plan_strips` runs **before** rayon divides anything, so all of it is on the critical path while the
4.9 G under it is not. `examples/strip_spans` prints this page's own plan: asked for sixteen strips
it is granted **eleven**, whose slowest holds **10.6%** of the estimated cost. Taking callgrind's
numbers per render — 271 M total, 12.7 M of it the example's ink sum, 24.7 M the planner — the
drawing's contribution to the critical path is about 233 M × 10.6% ≈ **24.7 M**, which is the
planner's own figure to three digits.

**The plan was half the critical path of a divided page**, and it is stated here as an estimate
built from two measured numbers rather than as a clock, because a clock on this machine measures
this machine (session 749).

## What the walk was doing

`unsplittable_rows` returns one `bool` per target row, `true` where a horizontal cut would re-state
a segment — ADR 0139's rule, and the reason strips are exact rather than approximately exact. It
computes it by walking every fill's, every clip's and every soft mask's path and calling `mark` for
each oblique segment. On page 101 that is **76 991 marks per render** over 3007 commands.

`mark` only ever *sets* a row. A dense text page is thousands of glyph fills over a few hundred
rows, and once a line of text has marked its own rows every later glyph on that line walks forty
control points to mark them again. Nine marks in ten on this page changed nothing.

## The change

One test in front of the walk:

```rust
fn settled(rows: &[bool], path: &crate::Path, at: Transform) -> bool {
    let Some(extent) = path.bounds(at) else { return false };
    let count = rows.len();
    let from = clamp_row(extent.min.y.floor() + 1.0, count);
    let to = clamp_row(extent.max.y.ceil(), count);
    rows.get(from..to).is_none_or(|span| span.iter().all(|row| *row))
}
```

**It is exact, not conservative, and that matters more than the saving.** `Path::bounds` is the
control hull — its own doc comment says so and says why — and `oblique_spans` reports y ranges of
those same control points, so every segment's `[top, bottom]` lies inside the path's `[ymin, ymax]`.
`floor`, `ceil` and `clamp_row` are all monotone, so every segment's mark range lies inside the
range tested here. Where the test passes, the walk would set no row that is not set: the vector
produced is the vector the unconditional walk produces, row for row.

`Path::bounds` is memoised in a `OnceLock` per path and mapped rather than re-walked wherever the
transform keeps the axes — the memo the hundred-and-sixty-third session added to pay for the strip
driver's own `Path::bounds` calls and ADR 0387 later measured — which is what makes the test cost
about 98 instructions against a glyph walk's several thousand. **This change is the second caller
that memo carries, and it is the first one on the serial half of the render.**

## What moved

Both arms built and run in one sitting, same binary path, same arguments. The before arm was
re-derived on this tree rather than quoted from ADR 0677 — session 757's own lesson — and run three
times against the after arm's two, because a parallel render's instruction count carries rayon's
scheduling with it and repeats here differ by up to 0.3%.

| | before | after | |
|---|---:|---:|---|
| ISO 32000-2 p. 101 ×20 | 5 412 167 781 / 5 428 689 008 / 5 426 590 074 | 5 046 315 493 / 5 030 698 863 | **−7.08%** |
| `tracemonkey.pdf` p. 1 ×10 | 2 370 062 770 | 2 232 441 225 | **−5.81%** |
| ISO 32000-2 p. 6 ×20 | 3 641 516 664 | 3 525 317 024 | **−3.19%** |
| `bug1721218_reduced.pdf` p. 1 ×2 | 3 661 400 384 | 3 622 806 372 | −1.05% |
| `issue12841_reduced.pdf` p. 1 ×5 | 8 717 134 836 | 8 698 566 957 | −0.21% |

Every page falls and none rises; the ink sum every run prints is identical in every pair. Inside
the profile:

| | before | after |
|---|---:|---:|
| `strips::segments`, inclusive | 448 348 680 (8.28%) | 56 526 340 (1.12%) |
| the whole planning prologue | 494 039 652 (9.13%) | 102 319 839 (2.03%) |
| `mark` calls, per render | 76 991 | 7 801 |
| `Path::bounds` calls the test adds, per render | — | 3 006, for 294 588 Ir |

The two pages that gain least are the two that should: `bug1721218_reduced.pdf` is 3490 `sh`
operators each stated over the whole page under a clip, so nothing settles until the rows are
saturated, and `issue12841_reduced.pdf` is two commands, each covering the page.

## Trap 13: a raster cannot calibrate this

**A byte-identical raster is not evidence here, and that is the trap in its purest form.** The
strips are exact by construction, so a planner that *forbade too few cuts* would still draw the
identical picture on this machine's thread count and lose only ADR 0138's edge coverage, on some
other page, at some other strip count. The instrument has to compare the row vectors.

A temporary switch on `settled` and a temporary example put the guarded walk beside the unguarded
one, row for row, over three pages of every document at 1×, 2× and 4×:

- **`doc/*.pdf` and the whole pdf.js corpus**: 3412 page-scales over 988 documents, **0
  disagreeing, 0 rows**.
- **The SafeDocs crawl**: about 18 000 documents and about 119 000 page-scales before the run was
  stopped for the gate sequence, **0 disagreeing, 0 rows**.

Three planted defects were caught before that zero was believed, each on the pdf.js corpus's 3412
page-scales:

| planted | disagreeing page-scales | rows |
|---|---:|---:|
| `any` for `all` — skip where *some* row is marked | 1833 | 161 864 |
| the tested range shrunk by one row at each end | 928 | 4 848 |
| the transform forgotten, testing the untransformed hull | 1176 | 38 492 |

Neither the switch nor the example is committed, for ADR 0667's reason: a differential is an
instrument for a change, not a second implementation to keep.

What **is** committed is the discriminating case as a unit test.
`a_shape_reaching_past_the_rows_already_marked_is_still_walked` builds two wedges whose hypotenuses
are the only oblique segments either has, the second starting inside the first's rows and reaching
past them, and asserts the exact row set. Against the planted `any` it fails at row 20, naming the
row.

## What this leaves

`Rect::from_points` is 4.21% of the page and 113.8 M of it is `tiny_skia_path::Path::transform`
re-deriving a path's bounding rectangle from all of its points after `PathBuilder::finish` has
already derived one — 62 100 calls at 1832 instructions each. Building the path pre-transformed
would remove one of the two walks. It is **not taken here** and no price is claimed for it: it
would replace `tiny_skia::Transform::map_points` with `pdf_render::Transform::apply` at every
control point, and two spellings of an affine map are not obliged to produce the same `f32`. It is
a departure to measure, not a saving to assume.
