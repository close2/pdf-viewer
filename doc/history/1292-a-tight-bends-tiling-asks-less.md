# 1292 — A tight bend's tiling asks less, and a stroke it tiles keeps its integral

Batch forty-five, performance slot. No ledger row moved (the contract named none; §8.4.3.2 stays
`implemented`). ADR 1421. `crates/render-cpu/` was not opened.

## Measured first (own exports and target dirs, `md5sum`-distinct; probes in exports only)
- Pieces are two per flattened segment (body + join) on all three pages; `bug1743245.pdf` 599
  tight strokes, 320 pieces mean, 1 273 max; Type 3 135 (3 subpaths), 221; `issue14415` 793, 70.
- Tight iff `hw > R·cos(θ/2)`: the half-width against the curvature radius, not the flattening;
  so no run of chords merges into one convex piece. Of the tiling's 7 172 M on `bug1743245`: the
  separation questions 3 653, the cutting 2 253, the scan 1 136; the fill then spent its sweep on
  598 of 599 tiled strokes.

## Kept (M Ir, one `frame_budget` round, Type 3 / bug1743245 / issue14415)
- Box of what is left prefilters earlier pieces: +17 / −775 / −11.
- A stroke whose every piece is tiled is `tiles`: +1 / −71 / −14.
- Fragments rewritten through reused buffers, boxes side by side: −270 / −1 105 / −52.
- Split returns a side without copying; box re-taken only after a cut: −34 / −59 / −3.
- Untiled pieces asked whether they stand apart (`the_rest_stand_apart`): +3 / −422 / 0.
- Total 2 087 → 1 803, 9 535 → 7 105, 1 393 → 1 314. Pinned turn/step ms: Type 3 11.16/12.80 →
  9.38/10.88; bug1743245 60.07/58.60 → 43.00/39.73; issue14415 11.74/6.38 → 11.24/5.44.

## Dropped
Sides cached per split (+52 / +213 / +11), carried per edge (+3 / +116 / +2); no tiling with the
sweep unbounded (3 904 / 6 974 / 2 206: cheaper on one page, 87% and 58% dearer on two).

## Exactness
Corpus first pages hashed against the tree before: 1× 0/958, compute 0/958, 4× 1/957 —
`issue19971.pdf`, 6 pixels one level, one stroke vouched for by the stand-apart check; the
pieces' clipped area is 23.50 levels there, so the kept 24 is the set's value (habit 53).

## Files
`raster/crates/raster-gpu/src/raster/stroke.rs`, `stroke/disjoint.rs`, `tests/curve_join.rs`,
`tests/stroke_set.rs`, `doc/performance.md` (two rows, their note), `doc/todo/36` (one cell), ADR 1421.

## Gates
raster-gpu nextest 619/619, render-raster 93/93, clippy both 0, conformance 0; raster_golden moved
0; corpus 1× 959/2 and 4× 958/0, thread count 0 each; headless_gpu 39/39; launch_path 26 banded,
0 outside. Left: the tiling is still most of the Type 3 page's encode (864 M against its floor);
each placement of a cell tiles again (the encoder's cache); several subpaths still ask.
