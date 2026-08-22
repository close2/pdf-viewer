# 646 — The nine pieces the library already had

`doc/todo/11` item 7, taken: an axis-aligned rectangle's edge is drawn at the product of its two
overlaps rather than rounded to a quarter of a pixel, for its **fill** and for its **clip region**
alike. Session 643 priced the cure at nine `scan::fill` calls where there was one and declined it on
that price. **The price was wrong**: `tiny-skia` already contains a rectangle scan converter that
walks those nine pieces in one call, and the change is *cheaper* than what it replaces.

Date: 2026-08-22. ADR: **0476**.

## The geometry, and where it comes from

§10.7.4 defines a device pixel as a product of two half-open intervals and gives a filled shape the
same form. An axis-aligned rectangle is a product of two intervals too; the intersection of two such
products is the product of the two intersections, and the area of a product of intervals is the
product of their lengths. So a rectangle's coverage of a pixel is `overlap_x · overlap_y`, exactly,
at every placement — arithmetic out of the clause rather than a constant anybody tuned.
`pdf_render::edge` states it (`device_rectangle`, `rectangle_coverage`), in the crate both backends
read, for trap 2's reason.

## The two sites, and the one that was not in the plan

A **fill** goes to `tiny_skia::PixmapMut::fill_rect`, which is the same library's rectangle
converter: one interior run, four edges, four corners, 8.8 fixed point, no supersampling.

A rectangular **clip region's** mask is written from the same closed form by hand, because a `Mask`
has only `fill_path`. That was not in the plan and is not optional: §10.7.4 says the clipping region
"consists of the set of pixels that would be included by a fill operation", and the first version of
this change measured only the fill. `clip_intersection.rs` failed within the round, by **26 levels of
255** — a mark painted at its exact area under a region still measured to a quarter breaks the same
paragraph's `S ∩ C = S`. A gate written for one sentence caught the violation of another sentence of
the same clause.

## What it costs

`callgrind_rasterise`, twenty repeats, `RAYON_NUM_THREADS=1`, two passes agreeing to 0.001%, both
arms built here in one sitting. Machine load 44.72 / 34.17 / 40.23 at the start, 29.40 / 30.47 /
37.03 at the end; the counts are load-immune and this round can say so rather than assume it,
because the same six figures came back within 0.01% from a pass taken at load 103.

```text
  ISO 32000-2 p101 (text)                     5,420,405,148 -> 5,396,982,320   -0.43%
  ISO 32000-2 p6   (303 runs, page-wide clip) 3,993,290,492 -> 3,608,520,927   -9.64%
  colors.pdf p1    (sixteen rectangles)       1,758,375,215 -> 1,617,776,872   -7.99%
```

The launch clock cannot see this and that is structural: page one goes to the GPU, so `render-cpu`
is not on the launch path, and a wall-clock A/B on a machine carrying three other rounds would have
measured the neighbours.

**One de-optimisation was found and paid inside the round.** The first `mask_rectangle` asked the
closed form per pixel over the whole region, which on `colors.pdf`'s page-wide clip cost **+33%** of
the page. Writing the interior as a run is what turns +33% into −7.99%.

## What moved

- `edge_coverage_ladder`: `render-cpu` answered 0, 0.2510, 0.5020, 0.7529, 1.0000 and now tracks the
  fraction to a level of 255 on both backends and both axes, at all twenty-one rungs.
- **The oracle 907/66 → 908/65**, measured both ways here. `issue21346.pdf` page 1 left the
  contradicted list, emptying `CONTRADICTED_COINCIDENT_CLIP_EDGES`; its edge went **0.306 → 0.469**
  of the mark. It does not pay item 4 — the two edges stand in the ratio `(0.75/0.827)^4.4`, so four
  to five of that page's seven statements of one rectangle are still products.
- **`render-quorra` 933/23 → 933/22**: `issue18823.pdf` left, the processor moving to the device.
  Three of that four-page population stay, because theirs is the clip *product* and not the
  coverage.
- **`colors.pdf` is still contradicted and that is the confirmation.** 643 predicted, from the file's
  own arithmetic with no code and no renderer, ssim 0.98772 and 0.98001 for a rasteriser painting
  precisely the covered area. The gate now measures **0.9879** and **0.9802**, against bounds of
  0.9886 and 0.9840.

## The ink sweep

`doc/todo/00` step 7, both arms, all 786 ambiguous pages: **645 identical to a thousandth, 141
moved, median 0.0040 of 255, 98 of them toward the lightest reference and 43 away**, summed distance
139.109 → 137.985. **19 pages at or past −1 before and after**, all diagnosed; negative head
byte-identical. The two largest movers are `bug1844583.pdf` −0.372 and `bug1844576.pdf` −0.298, both
widget appearances, both toward the references, both opened and looked at.

## Gates

The whole of `doc/todo/02` §2 including the fuzz line; §5's binaries rebuilt and installed.
`spec-errata emit` on the base standard was run first and confirms 643 rather than assuming it: no
annotation against §10.7.4, and the only erratum in the §10.7 family is **#371** on §10.7.2's
flatness.

## Corrected

§10.7.4's ledger row, `doc/todo/_scan-conversion.md`'s departure (1) and its
`CONTRADICTED_TIGHT_CONSENSUS` bullet, `doc/todo/11` items 4 and 7, `doc/todo/README.md`'s row,
`oracle.rs`'s emptied group and the paragraph above it that called the page contradicted, and
`render-quorra/tests/corpus.rs`'s `/BBox` population note.
