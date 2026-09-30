# 1298 — A stroke of several subpaths is one set, and a tight bend's scan keeps its box test

Batch forty-six, raster slot. No ledger row moved (§8.4.3.2 stays `implemented`). ADR 1431.
`crates/render-cpu/` was not opened.

## Measured first
- Probe, one `frame_budget` round on `bug1743245.pdf`: the scan looks at 31.2 M pairs, 1.9 M
  boxes meet, 1.74 M are asked; 7.7 M fragments tested, 1.2 M cut. The questions are already
  asked only of pairs that meet; what is quadratic is the bend's own overlap.
- 830 strokes of more than one subpath on 43 corpus first pages reach the fill's set question,
  357 past its bound (1×); at 4× 759, 150.

## Dropped
The sweep for the tiling's scan and `takes_part`: +124 / −25 / +57 M (Type 3 / bug1743245 /
issue14415) at best; vouching only for subpaths that stand apart: +169 / +3 / +45 M.

## Kept
A stroke's pieces are tiled across subpaths where pieces of two subpaths overlap (`tile_stroke`).
Asked after: 211 (141 past the bound) at 1×, 172 (86) at 4×. 8 pages move at 1× and on the
compute lane, 5 at 4×; every moved stroke looked at: within 0.500 of the exact clipped area where
the pieces are disjoint (up to 119 levels before), nearer the sampled set everywhere else.
Cost: 1 865 → 2 170 / 7 420 → 7 407 / 1 356 → 1 479 M; pinned turn ms 9.46 → 11.28 / 46.09 →
44.34 / 11.62 → 12.29. `convex.rs` split out of `disjoint.rs` (584 lines).

## Files
`raster/crates/raster-gpu/src/raster/stroke.rs`, `stroke/disjoint.rs`, `stroke/convex.rs` (new),
`stroke/sweep.rs` (new), `tests/stroke_set.rs`, `tests/curve_join.rs`, `doc/todo/36` (one cell),
ADR 1431.

## Gates
rustfmt 0; clippy raster-gpu, render-raster 0; raster-gpu nextest 629/629, render-raster 93/93;
conformance 0; raster_golden moved 0; corpus 1× 960/1, 4× 958/0, compute 959/1, gpu 959/2, thread
count 0 on each; headless_gpu 39/39.

## Left
Tile across subpaths only where the fill's question is past its bound — the choice needs the job's
region, in `encode/parallel.rs` (ADR 1431 section 5); the Type 3 cell's per-placement tiling.
