# 1445 — A stroke's expansion is made once for its placements, and tiling across subpaths is not traded for the fill's question

Status: accepted. Session 1305. Answers ADR 1375 section 2's and ADR 1421 section 6's "the Type 3
cell is tiled again at every placement; that is the encoder's to cache", and measures and drops
ADR 1431 section 5's "tile across subpaths only past the fill's bound". Amends ADR 1397's and
ADR 1421's stroke construction in one respect: a stroke is expanded under its device transform's
linear part and then translated. Keeps ADR 1375, ADR 1407 section 2 and ADR 1431 section 2.
Supersedes nothing.
Context: ISO 32000-2 §8.4.3.2, §10.7.4; `CLAUDE.md` principle 2 and the rule on optimisations;
`doc/habits/measuring.md` 50, 52, 53, 55, 56, 57; traps 50, 54, 58, 66, 71, 73, 78;
`doc/questions/A76`.
Code: `raster/crates/raster-gpu/src/encode/expansion.rs` (new: `Expansion`, `Expansion::placed`,
`Slot`, `expansion`, `Expansions`, `HELD_BUDGET_SHARE`), `encode/parallel.rs` (`Job::sharing`,
`rasterise`), `encode/stroke.rs` (both paths read `expansion`), `encode.rs` (`Encoder::expansions`),
`raster/stroke.rs` and `raster.rs` (`stroke_polylines` is the fixtures' now).
Tests: `raster-gpu/tests/encode_threads.rs`
(`a_stroke_placed_many_times_draws_what_it_draws_placed_alone`: at the default budget and one whose
share keeps nothing, and at five thread counts). Planted: an expansion made at the first asker's
placement and moved by the difference fails it at two threads.

**I did not open `crates/render-cpu/`.** Every expected value below is a closed-form area or a
measurement of raster against itself; the oracle's mean error is printed beside the pages as
evidence only.

## 1. What was measured first

A probe export (a line per stroke job, per path) on one `frame_budget` round of
`ContentStreamNoCycleType3insideType3.pdf`: **every stroke went through `encode/parallel.rs`,
none through `encode/stroke.rs`'s own path** (trap 78). The turn expands 144 strokes of 24
outlines under one linear part — 16 outlines at 7 placements, 8 at 4, each of 2 subpaths and 342
or 795 pieces — and the step 72 under another. No two placements share a translation. Callgrind,
one round, whole process, M instructions (`md5sum`-distinct binaries, own target directories and
exports): the Type 3 page 2 126.9, of which `rasterise` 1 594.1, `stroke_pieces` 1 402.2 (66% of
the process), `disjoint::tile` 1 191.2, `fill_mask_settled` 151.9. p101: 866.0, no strokes.
`issue14415.pdf`: 1 457.2, `stroke_pieces` 439.2, few repeated shapes.

## 2. The expansion, made once per shape (§8.4.3.2)

> stroking a path shall entail painting all points whose perpendicular distance from the path in
> user space is less than or equal to half the line width

A translation carries that set onto the translated path's, and the device width is the linear
part's. So `Expansion::of` flattens and strokes under the linear part with no translation and
`Expansion::placed` adds the translation to every point. **Every stroke is built so**, shared or
not, on both paths, so a placement that reads another's expansion and one that made its own are
the same bytes whatever the thread count or the room. The key is the atlas's (ADR 0009): outline,
linear part's bits, the stroke's bits. It lives one frame; a slot is made only for a shape the
scene places twice or more (`Expansions::of` counts before the walk, as the census does for fills);
the slots together hold at most a sixteenth of `max_frame_bytes`, past which a placement makes and
drops its own. A cache across frames was not built: a zoom step is a new linear part and a repaint
does not encode.

Callgrind, base → this: **Type 3 2 126.9 → 988.8** (`stroke_pieces` 1 402.2 → 243.0, `tile`
1 191.2 → 201.4, `fill_mask_settled` 151.9 → 148.1); p101 866.0 → 866.1; `issue14415.pdf`
1 457.2 → 1 460.9 (+0.26%, a placement that owns its pieces moves them in place; copying them
cost +26 M). `frame_budget`, pinned minimum of three runs of five, one sitting, load 5.2–6.4, ms
budget / encode (unpinned in brackets): Type 3 turn **11.48 / 10.03 → 5.98 / 4.56** (9.48 → 6.15),
step **11.65 / 11.02 → 6.01 / 5.37** (8.86 → 6.00); p101 turn 6.69 → 6.84 (7.83 → 7.68), step
1.45 → 1.40; `issue14415.pdf` turn 11.56 → 12.11 (12.34 → 12.76), step 6.42 → 5.77 (5.68 → 6.37) —
the last two within the runs' spread, as the instruction counts say. **Kept.**

## 3. It is not byte-identical, and cannot be

The brief asked for the old bytes. The stroker's arithmetic done at the origin rounds differently
from the same arithmetic done six hundred pixels away, and no cache that serves two translations can
reproduce both. `render-raster --test corpus`, each arm its own export, digests per page (ADR 1443's
column): **52 pages move at 1×, 80 at 4×, 52 on the compute lane and 43 on the GPU lane at 1×**; the
gate's counts are unchanged on every one (960 agree / 1 / 6 / 7 at 1×, 958 / 0 / 8 / 8 at 4×, 959 /
1 / 7 / 7 on the compute lane, 959 / 2 / 6 / 7 on the GPU lane) and one encode thread against many
held at 0 on all four. Of ADR 1431's eight corrected pages, `issue12823.pdf` holds at both scales
and the other seven move at one or both, by the rounding below. Every moved pixel of six pages was
read against the set (habit 53; the pieces at the origin, shifted in `f64`; a stroke whose pieces
tile summed exactly by clipping each to the pixel, one whose pieces overlap sampled 512 × 512):

- **Where the pieces tile, the new value is the nearer one**: `issue17492.pdf` 925 of 925 pixels,
  `issue9972-1.pdf` 61 of 61 (exact ties, 127.5, now rounded up as ADR 0005 states),
  `annotation-line.pdf` 4 of 4, `bug1743245.pdf` 16 of 16 (worst 248 → 252 against 251.97),
  `issue14415.pdf` 5 of 6 (127 → 135 against 135.27); the Type 3 page 8 of 18, all within 0.05 of
  a tie. Computing at the origin keeps the coordinates where `f32` is densest and rounds once.
- **Where they overlap, the fill's question is past its bound in both arms**, so both integrals
  count what the tiling left overlapping, and which fragments are left depends on the rounding:
  `bug1743245.pdf` 23 pixels, 3 nearer (worst 232 → 243 against 231.85), `issue14415.pdf` 21, 10
  nearer (255 → 245 against 245.27). Neither arm is right there; ADR 1421 section 6's bound is.

## 4. Tiling across subpaths only past the fill's bound, measured and dropped

Built in an export: where subpaths' pieces overlap, each subpath is tiled alone and the fill is
asked of those pieces for the job's region; within the bound its answer stands, past it the
stroke is tiled whole as ADR 1431 does. Callgrind against base: **Type 3 2 126.9 → 2 939.7**,
`issue14415.pdf` 1 457.2 → 1 653.3, p101 unmoved. On the Type 3 page 70 of the 132 crossing
strokes are past the bound (probe), and every glyph has tight bends, so asking needs each
subpath tiled alone first (`each_alone` 827.5 M) and a stroke past the bound is then tiled again
whole (`tile` 1 191.2 → 1 771.2). Under section 2 the whole tiling is paid once per shape and the
fill's question would be paid per placement, which is dearer still. Not built on either path, not
hashed.

## 5. Left

The Type 3 page's turn is now `fill_mask_settled` and the one expansion per shape; the rim
pixels of strokes past both bounds are the fidelity item, and ADR 1421 section 6 names it.
