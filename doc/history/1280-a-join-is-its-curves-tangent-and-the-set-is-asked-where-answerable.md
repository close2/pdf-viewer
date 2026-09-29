# 1280 — A join is its curve's tangent, and the set is asked where it can be answered

Batch forty-three, raster slot. No ledger row moved (§8.4.3.4 and §8.5.3.3 were `implemented`;
raster's side now meets them more exactly and more cheaply). ADR 1397; feedback section 56
appended. `crates/render-cpu/` was not opened.

## Built
- **Cost** (`fill/topology.rs`, `stroke.rs`, `encode/parallel.rs`): a subpath's inside point and
  ray test in `f64` (a hairline cap at x ≈ 1 000 was read as nested: 29 042 walks a frame on
  `issue12810.pdf` → 480); the sweep sorts packed `(key, index)` integers; `stroke_pieces` says
  when its pieces tile (one straight segment, or closed convex with every corner cut) and such a
  stroke keeps its integral unasked.
- **Joins** (`flatten::Tangent`, `flatten_stroke`, `stroke/centre.rs`): each piece ends square to
  a curve's tangent where the chord carries it; joins, miter limits and inner cuts use those
  directions; a point whose tangents turn no more than its chords is drawn as a flattened vertex.

## Measured (callgrind, 3 frames, M Ir; arms md5-distinct, own target dirs)
`issue12810` 12 491 → 4 644, `bug1743245` 9 790 → 8 980, `issue14415` 1 078 → 964, `issue20513`
429 → 406, `issue9418` 2 564 → 2 577, page 101 158.0 → 152.6. Forty costliest pages, one frame:
13 244 → 10 131 (the construction 4 637 → 1 524 over the never-ask arm's 8 607). Dropped: skipping
pairs inside a convex subpath (+0.1–1.3%). Wall clock could not resolve it (the oracle moved 27%).
Join fixture (`curve_join.rs`, 90° arc→line, 1×–8×, both ways): miter/bevel ≤ 1 step per pixel;
chord-planted 85/94/194/200. Hook 557.686/560.729/562.384/564.297, unchanged: one cubic, no join.

## Gates
Tier 1: rustfmt (own files), clippy `-D warnings` raster-gpu/raster-scene/render-raster/
raster-pages exit 0; nextest raster-gpu+raster-scene+raster-pages 664/664, render-raster 93/93;
conformance exit 0. Tier 2: corpus 1× 959/2 exit 0; 4× cpu 958/0, gpu 957/0, compute 958/0, each
exit 0; `render-gpu --test headless_gpu` 39/39; `compute_lane.rs` on RADV passes. `raster_golden`
untouched. Archetypes re-recorded with reason (artwork 67/380/3 555 182, drawing 252 texels).

## Left
ADR 1375's tiling is the largest price left: off, the forty pages read 6 812 M and the corpus
still 959/2, but a stroke_set test fails by design and the fill's bounds on tight bends are
unmeasured. The walk's inline and residue-clipped fills re-ask per frame (Type 3 page 96 regions);
a three-valued outline cache would carry a completed "yes" exactly. ADR 1395's stroke weight (24.9)
was measured on HEAD and wants re-taking on the merged tree.
