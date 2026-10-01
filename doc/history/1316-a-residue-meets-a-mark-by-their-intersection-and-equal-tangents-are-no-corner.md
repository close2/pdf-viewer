# 1316 — A residue meets a mark by their intersection, and equal tangents are no corner

Raster, from the clauses alone: ADR 1456's priced exact meet, 1310's equal-tangent join, ADR 1443's
coarse-tiling question. ADRs 1467, 1468. No question. `crates/render-cpu/` was not opened (A76).

**The exact meet (ADR 1467), built on both paths.** Where the mark's and the clip's bytes are both
fractional, the pixel's value is the area of their intersection computed from both sets' edges
(`raster/meet.rs`: bands, a ray-count winding per set, trapezoids in `f64`); `min` stays everywhere
else and on the image lane. The fan-out returns a residue job's polylines to its commit; the chain's
edges are cached per frame under their own budget. Fixtures: a fill and a stroke under a 64-gon held to
the closed form within one rounding on three arms (fan-out at 1 and 4 threads, walk tile), byte-equal;
both failed under `min`. Corpus 1×: 56 858 doubly-fractional pixels, 43 066 lowered by 23 levels each;
9 pages moved, 8 further from the oracle by under 0.01 (expected: the oracle meets bytes), 0 differ.
Cost 0.66 s over the corpus, 0.64 s of it one page.

**Equal tangents (ADR 1468).** Measured on a circle of four cubics: miter and bevel drew 4–8 pixels
apart from round, up to 8 above and 16 below; an S-curve drew none. Fixed: a point where the path's
arriving and leaving directions agree to `f32`'s own uncertainty is joined round. 12 corpus pages moved
at 1×, all equal to the oracle to four places.

**Coarse tiling for `issue14415.pdf`:** derived inexact (the coarse union misses the fine rim's sliver
and covers past the inner one); not built.

**`issue1905.pdf` after `bug1721218_reduced.pdf` (ADR 1467 section 4).** Refused in sequence, fits
alone, on HEAD's raster-gpu too: the glyph atlas, full of the earlier page, sent glyph tiles to the
scratch sheet and its packing grew past the budget. A frame refused room among entries it never used
now resets the atlas and is encoded again; test planted and watched failing. `bug1721218_reduced.pdf`'s
3.7 s is ADR 0049's per-id residue path (97% of the frame), priced in section 3.

**Gates.** Corpus 1× and 4× cpu, 1× compute and gpu, headless_gpu, raster_golden, launch_path: see the report. Proposed habit: a refusal that appears in a corpus walk is re-run alone and
with its predecessor before it is attributed to a change.
