# 1444 — A residue clip meets a mark as a set, and a sliver is no part of a tiling

Status: accepted. Session 1304. Amends raster's ADR 0074 ("the product stays" at the mark's site,
now `min` on the CPU residue and the image lane's residue) and completes ADR 1435's rectangle
decision for the one site it left; amends ADR 1431 section 5 (its sliver, answered). Answers
`doc/QUORRA_FEEDBACK.md` section 59, ask 2. Supersedes nothing.
Context: ISO 32000-2 §8.5.4, §10.7.4; ADR 0030; habit 53; traps 13, 26, 67; `doc/questions/A76`.
Code: `raster/crates/raster-gpu/src/encode/coverage.rs` (`meet_residue`, `residue_meet` — a
targeted edit in a file `encode/` owns), `encode/parallel/commit.rs` (the one call renamed),
`encode/clips.rs` (wording), `shaders/image.wgsl` (`shape_at`), `raster/stroke/convex.rs` (`sliver`,
`Convex::subtract_from`).
Tests: `raster/crates/raster-gpu/tests/residue_meets_a_mark_as_a_set.rs`;
`raster/src/raster/tests/stroke_set.rs` (every piece's sign read, however small).

**`crates/render-cpu/` was not opened.**

## 1. The residue, from the clause

§10.7.4: "Subsequent painting operations shall affect a region that is the intersection of the set of
pixels defined by the clipping region with the set of pixels for the region to be painted." A residue
is a clip rasterised to a coverage byte, and a mark's coverage is another; in one pixel the
intersection's area lies in `[max(0, s + c − 1), min(s, c)]` and is `min(s, c)` wherever one set holds
the other there. Two bytes say how much and not where, so no function of them is the area. The same
paragraph says which side an estimate may take — "[t]he area covered by painted pixels shall always be
at least as large as the area of the original shape" — and of the functions of `(s, c)` never below
the intersection, `min` is the least, because nested sets attain it. The product is below it where
the edges coincide (`0.6 × 0.6` for 0.6) and above it where the sets miss each other in the pixel.

## 2. Measured against the closed form

A regular 64-gon clipping a lattice of 99 small rectangles, every expected value a polygon's area in
a pixel (`f64`, clipped to the pixel). Over the 48 pixels where both coverages are fractional: the
product misses by 15.4 levels of 255 on average, from 44.1 below to 42.9 above; `min` is 22.1 above on
average and never below. `issue2177.pdf`'s worst tile (32, 224), against ADR 1435's reference (128 ×
128 samples), with ADR 1443's flattening in place: product 1.09 of 255 (signed 0.00), `min` 1.35
(0.51 heavy); page 0.187 and 0.197. Before either change the tile was 7.42 on raster and 8.47 on the
oracle — the flattening was its cause, and what `min` adds is a quarter of a level on the side the
clause names. **Decided by the clause's direction, and the cost is written down**: at crossing edges
the product is the closer estimate on average and wrong on both sides; `min` is never below the set,
exact at a coincident or nested edge, and the same rule every other coverage site in raster takes.

Built: `residue_meet` is `min` per pixel on the CPU residue, and `image.wgsl` meets its residue byte by
`min`. The fixture's coincident-edge pixel reads 153 where the product drew 92 on both lanes, and the
lattice holds every pixel to `min` of the closed-form coverages and to at least the intersection; all
three fail under the product (watched, trap 13). What would do better than a bound is the area of
`S ∩ C` in the rim pixels where both are fractional — the mark's and the clip's edges walked together,
as `fill::exact` walks one path's — which needs the clip's polylines kept beside its region; left.

## 3. The slivers

Where the tiling cuts a piece along an edge of an earlier one, the two pieces' shared corner was
computed twice and can differ by one `f32` ulp; the cut then passes a vertex one ulp away and leaves a
needle triangle of that width — areas from 1e-11 to 7e-7 against pieces of tens, either sign — or,
with a vertex exactly on the line, a fragment of zero area. The rings fixture held 2 pieces of 461 wound
against the rest. A side is taken in `f64` from `f32` points, far below the grid; the crossing point is
rounded back to `f32`, moving it by less than one spacing `δ = m · f32::EPSILON`, `m` the fragment's
largest coordinate. **A fragment whose area taken with its piece's winding is at most `δ` times its
box's perimeter — which bounds a convex polygon's own, so its width is nowhere above that rounding —
is dropped**, in `f64` (trap 67): at most `δ` along each side of the box, 6.1e-5 of a pixel at a
coordinate of a thousand. The box and not the perimeter because the tiling asks it of every fragment:
a square root per edge cost `bug1743245.pdf` 14% of a frame's instructions, and with the box the
whole round costs that page 1.6% (ADR 1443 section 4). The fixtures now read every sign: no piece is wound
against the rest.
