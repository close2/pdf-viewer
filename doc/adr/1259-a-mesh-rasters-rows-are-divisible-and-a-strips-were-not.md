# 1259 — A mesh raster's rows are divisible, where a strip's were not

Session 1211. Status: **accepted**. Changes `crates/pdf-render/src/shading.rs`'s
`MeshRaster::build`. Amends nothing; it is the counterpart of
[ADR 0138](0138-the-strip-that-clipped-a-curve.md), which found the same division unsound one
layer up and said why.

## 1. What the frame spends on a mesh, measured

`examples/frame_budget` (ADR 1260) puts `personwithdog.pdf`'s zoom step — the page placed at twice
the magnification against caches the first placement filled, on the moved-view lane the window
actually takes — at **18.04 ms**, of which **8.22 ms is `render-raster`'s scene walk**. That is
this tree's own code, it is 99% of one 120 Hz refresh for the shadings of one page, and
`FrameCost::handover` says 0.0 of it is spent inside raster's `upload_*` calls, so it is not the
boundary either.

What is in it: the page states §8.7.4.5.7 type 7 patch meshes, fifteen of them, each rasterised
into a device-resolution image by `MeshRaster::build` — the shared rasteriser all three backends
draw a mesh with — and a magnification re-rasterises every one, because the raster is this view's
pixels (ADR 0702). Each is between **1 430 and 141 728 pixels** and carries **280 to 1 086
triangles**.

## 2. Why the rows divide here and did not there

ADR 0138 cut a *rasterisation* into strips, gave each a transform of its own, and found three of
eleven scenes moved — `curves` by 3 982 bytes and as much as 64 of 255. The mechanism it isolated
is coverage: an antialiasing rasteriser computes a pixel's coverage from geometry that crosses the
cut, so a strip boundary is a boundary in the *arithmetic*.

`Triangle::paint` has no such arithmetic and says so in its own doc comment: "[a] pixel belongs to
the triangle when its *centre* does — no antialiasing and no partial coverage". A pixel's colour is
therefore decided by the triangles covering that one point, in the order they are painted, and
nothing about a neighbouring row enters it. So a band — a contiguous run of whole rows, painted by
the triangles that reach it, in the order the file states them — produces the bytes the serial walk
produces, and which thread paints which band cannot change one.

That is an argument, so it has a test rather than a claim:
`both_arms_of_the_division_paint_the_same_bytes` rasterises two overlapping triangles both ways
over a raster above the floor and demands byte equality. It is calibrated against the defect
(trap 13): painting a band's own triangles in reverse order fails it.

## 3. What it bought, and what the arms were

`examples/frame_budget`, `personwithdog.pdf` page 1, the zoom step's `scene` stage, each figure the
minimum of three rounds on a device of its own, the arms alternated in one sitting on one machine:

| arm | `scene` | the step's whole budget |
|---|---|---|
| walked (the revision before this one) | 8.22 ms | 18.04 ms |
| divided (this one) | **3.66 ms** | 16.83 ms |

and, in an earlier sitting under a load average of 11 to 16, with the floor switchable so that the
two arms shared one binary, four alternating pairs: **every divided sample (3.35 to 4.90 ms) below
every walked one (6.50 to 14.83)**. A third pair taken at a load average of 15.7 put the divided
arm at 4.41 against the walked arm's 8.22 at a load of 6.4 — which is the separation surviving a
machine two and a half times as busy.

The page turn's `scene` moves much less (3.39 → 3.09 ms): at 1× the same meshes are a quarter of
the pixels and much of that stage is the 48 outlines it hands over.

## 4. The floor is 4 096 pixels and the first version's was sixteen times that

The obvious figure was `crate::paint`'s `PARALLEL_FLOOR` — 65 536 source samples, the measured
crossover for an area-averaged image reduction — and taking it made the change **do nothing**: at
65 536 not one of this page's fifteen rasters divided, and the frame was the serial frame to within
its noise. The two quantities are not alike. A reduced image is *one* raster the size of a
photograph; a mesh shading is *many* rasters the size of the shape each fills.

So the floor is at **4 096 pixels — a 64 by 64 raster**, which divides everything this witness
states and leaves a page of very small meshes the serial walk. Dividing every raster regardless
(floor 0) measured no better than the floor does on this page, and the crossover between them was
inside this machine's noise on the afternoon it was taken; where the measurement is decisive is
that 65 536 is far too high.

## 5. What it costs in readability, and what is paid for it

`CLAUDE.md` asks an optimisation for a benchmark and a comment, and asks the clear construction
over the clever one. What this costs: two small types (`Placed`, `Band`), a free function
`rasterise` whose `divided` argument exists so the two arms can be held against each other by a
test, and one bucketing loop — the triangles are sorted into their bands once rather than every
band asking every triangle, because a patch mesh tessellates into hundreds and that question would
grow with the pool.

What it does not cost: `raster_golden` is unmoved (the two pages that move in this worktree move
with this change reverted, and neither states a mesh), and `render-raster --test corpus` agrees
with the CPU oracle on the same 942 pages as before.

## 6. What this does not do

It divides the *paint*. `PatchMesh::tessellate` is still serial, and at a large magnification it is
a growing share of what is left — each patch is independent and produces its triangles in order, so
the same argument would carry, with its own measurement. `doc/todo/36` records it as the next thing
on this stage.
