# 1456 — The exact meet of a residue and a mark is priced, and `min` stays

Status: accepted. Session 1310. Answers ADR 1444 section 2's "What would do better than a bound is the
area of `S ∩ C` in the rim pixels where both are fractional … left" by pricing it. Keeps ADR 1444's
`min`. Supersedes nothing.
Context: ISO 32000-2 §8.5.4, §10.7.4; ADR 1395 (the fan-out's commit), ADR 0049 (the region cache);
habit 52; `doc/questions/A76`.
Code: none changed. The sites priced are `raster/crates/raster-gpu/src/encode/coverage.rs`
(`residue_meet`), `encode/clips.rs` (`residue_intersection`), `encode/parallel.rs` (`rasterise`) and
`encode/parallel/commit.rs` (`commit_sheet`).

**`crates/render-cpu/` was not opened.**

## 1. What the clause asks, and what the exact answer needs

§10.7.4: "Subsequent painting operations shall affect a region that is the intersection of the set of
pixels defined by the clipping region with the set of pixels for the region to be painted." In a pixel
both sets cut, the intersection's area is a function of where their edges run, not of their two bytes,
so the exact value needs both shapes' edges in that pixel at the moment they meet: the area of
`{w_S inside} ∩ {w_C inside}` over the pixel, by a slab sweep over the two edge sets with each set's
winding carried in from the left, under each link's own rule. Three things the tree does not hold:

- **The mark's edges at the meet.** ADR 1395's fan-out rasterises a mark on a worker and the commit
  meets its residue on the walk's thread with only the tile (`parallel::rasterise` returns a mask, and
  `commit_sheet` calls `meet_residue` with it). Building the exact meet on the walk's own path alone
  would make a frame depend on how many threads drew it, which the one-vs-many gate refuses; carrying
  the polylines to the commit changes the job's result type for every residue-clipped mark.
- **The clip's edges beside its region.** ADR 0049's cache keeps a chain's region as coverage and
  flattens the chain once; the polylines are dropped. Keeping them is a second cache with its own
  budget beside `ResidueRegions`, indexed by row so a rim pixel finds its few edges among a page-sized
  clip's thousands.
- **The exact two-set area itself**, which no routine of `raster/` computes today: `fill` integrates
  one winding field.

The first two are in `encode/` files this round does not own, and both change shared types; the third
is new arithmetic with its own fixture. So the build is a round of its own, not a residue of this one.

## 2. What it can move, measured

Every residue meet of the corpus at 1×, counted inside `residue_meet` (habit 52's off arm read the
other way: what the step could change at most): 10 540 meets over 5.19 M pixels; **13 340 pixels**
(0.26%) have both coverages fractional, and in them the interval `[max(0, s + c − 1), min(s, c)]` that
holds the exact value is 43.5 levels wide on average, 580 266 levels in all — 2 275 pixels' worth of
area across 961 pages. That is the most the exact meet could take off `min` anywhere in the corpus at
1×; on ADR 1444's fixture `min` stood half that interval above the exact value (+22.1 on 48 pixels),
and on `issue2177.pdf`'s worst tile it was 0.26 of a level above the product, which ADR 1443's
flattening had already brought from 7.42 to 1.09. No page differs from the oracle at 1× or 4×.

## 3. Decision

`min` stays: the least function of two bytes never below the intersection, on the side §10.7.4's area
sentence names, and wrong only inside an interval this section has now counted. The exact meet is
built when a page shows a residue rim as its largest difference from the geometry, as the round that
owns `encode/`'s fan-out and region cache, with the slab sweep held to the 64-gon fixture's closed
form (0 ± rounding is the bar) and its cost on the fixture page and on the pages that meet a residue.
