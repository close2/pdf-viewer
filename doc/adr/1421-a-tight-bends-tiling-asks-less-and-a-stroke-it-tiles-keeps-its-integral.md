# 1421 — A tight bend's tiling asks less, and a stroke it tiles keeps its integral

Status: accepted. Session 1292. Amends ADR 1375 section 2 (its cost) and ADR 1397's `Stroked::tiles`
(a tiled stroke is vouched for); keeps ADR 1407 section 2 (the tiling stays). Supersedes nothing.
Context: ISO 32000-2 §8.4.3.2, §8.4.3.4, §10.7.4; `CLAUDE.md` principle 2 and the rule on
optimisations; `doc/habits/measuring.md` 45, 52, 53, 54, 55; traps 50, 56, 58, 62, 65, 67, 71;
`doc/questions/A76`.
Code: `raster/crates/raster-gpu/src/raster/stroke/disjoint.rs` (`disjoint`, `Tiled`,
`the_rest_stand_apart`, `Convex::apart_from`, `Convex::subtract_from_each`,
`Convex::subtract_from`, `Scratch`, `Line::split`, `Split`), `raster/stroke.rs` (`stroke_subpath`
returns `tiles || tiling.whole`).
Tests: `raster/tests/curve_join.rs` (`a_stroke_whose_pieces_tile_draws_the_same_without_the_question`
gains the narrow rectangle and a lead-in to a tight bend; `a_stroke_that_may_overlap_itself_is_not_vouched_for`
loses the narrow rectangle and gains a stroke crossing itself after a tight bend),
`raster/tests/stroke_set.rs` (`a_tight_bend_counts_its_rim_once_either_way` holds the kept integral
to the closed form too). Planted: without `the_rest_stand_apart` the lead-in fails; with
`apart_from` always true the crossing stroke is vouched for and fails.

**I did not open `crates/render-cpu/`.** Every expected value is the clause's closed form or a
measurement of raster against itself; no other renderer was consulted.

## 1. Where the tiling's instructions went, measured first

Callgrind, one round of `examples/frame_budget` (turn, warm, step), whole process, each arm its own
export and target directory, binaries `md5sum`-distinct. A probe export (stderr counts,
`#[inline(never)]` on the tiling's functions) gave the population and the split:

| page | strokes with a tight bend | pieces a stroke, mean / max | pieces a segment | tiled / fragments out, mean |
|---|---|---|---|---|
| `bug1743245.pdf` | 599 | 320 / 1 273 | 2.01 | 257 / 1 254 |
| Type 3 page | 135 (3 subpaths each) | 221 / 542 | 2.00 | 199 / 718 |
| `issue14415.pdf` | 793 | 70 / 202 | 2.00 | 42 / 103 |

`bug1743245.pdf`'s strokes by pieces: 32–63: 35, 64–127: 123, 128–255: 142, 256–511: 189, 512–1 023:
100, 1 024–2 047: 10. **A piece is a flattened segment's body or the join at its vertex — two per
segment on all three pages**, not one per curve or per bend: 314 on average is 157 chords. Of
`bug1743245.pdf`'s 7 172 M in the tiling, the subtraction's questions took 3 653 M (31 M calls in the
round, most answered by a box and returned unchanged), the cutting 2 253 M, the scan over earlier
pieces 1 136 M, and building the convex pieces 131 M; and after the tiling the fill still asked
ADR 1389's set question of 598 of the 599 tiled strokes and ran out of its sweep on every one.

## 2. Which pieces must be disjoint (§8.4.3.2)

§8.4.3.2 makes a stroke a set: "stroking a path shall entail painting all points whose
perpendicular distance from the path in user space is less than or equal to half the line width".
The fill integrates winding and clamps it, so the pieces must share no area wherever the fill does
not ask the set question (ADR 1375), and ADR 1407 found the set question bounded exactly where the
tight strokes are. Where a bend is tight follows from `inner_cut`: the cut lies `hw · tan(θ/2)` back
along each chord and must stay within half the shorter one. On a flattened arc of radius `R`
whose chords turn `θ` each, a chord is `2R · sin(θ/2)`, so the vertex is tight exactly when
`hw > R · cos(θ/2)` — **the half-width against the radius of curvature, to first order independent
of the flattening**. A bend with `R < hw` is tight at every vertex and its bodies reach past the
centre of curvature, overlapping pieces that are not neighbours; a run of such chords is not one
convex piece, and the subtraction is over convex pieces. So "tile by bend" has nothing to merge:
the runs that do not bend tightly already take no part (ADR 1375's seeds).

## 3. The levers kept, each measured against the arm before it (M instructions, one round)

| lever | Type 3 | `bug1743245` | `issue14415` |
|---|---|---|---|
| base | 2 086.8 | 9 534.5 | 1 393.1 |
| (a) an earlier piece whose box misses the box round what is left is passed over | +17.1 | −775.2 | −10.6 |
| (b) a stroke whose every piece was tiled is `tiles` | +0.6 | −71.3 | −14.2 |
| (c) the fragments rewritten in place through reused buffers; the boxes side by side | −269.9 | −1 105.2 | −51.5 |
| (d) a polygon on one side of a line is not copied; the box re-taken only after a cut | −34.3 | −58.5 | −3.3 |
| (e) the pieces the tiling did not reach are asked whether they stand apart (`the_rest_stand_apart`) | +3.2 | −421.6 | +0.1 |
| all, with a zero-length edge never read as separating | **1 803.1** | **7 104.8** | **1 314.3** |

(a) and (d) are exact by construction: a fragment's box lies inside the box round its siblings, and
`subtract_from_each` hands back unchanged what no box meets. (b) and (e) change only whether the
fill asks: pieces that share no area wind every point `0` or one value, and ADR 1397's integral is
then the set's area. (e) asks each untiled piece against every piece it could meet, by a sweep
bounded at `MAX_FRAGMENTS` comparisons; a convex fragment lies inside its piece, so a piece apart
from that piece is apart from its fragments. (d)'s row is two arms' difference, the carried side of
section 4 taken out. Per function, base → kept: `stroke_subpath` 1 155.5 → 873.8, 6 093.4 →
4 328.3, 329.9 → 285.7; `Topology::of` 120.8 → 120.4, 1 658.1 → 998.5, 245.5 → 213.6.
**The tiling's own floor** (habit 52, the tiling switched off in an export): 938.9, 1 790.5 and
1 014.1 M, so the tiling costs 864, 5 314 and 300 M where it cost 1 148, 7 744 and 379.

Latency, `frame_budget`, this tree against the tree before it in one sitting, minimum of three runs
of five rounds pinned to the fast cores (unpinned in brackets), load 2.4–2.8, ms:

| page | row | before | after |
|---|---|---|---|
| Type 3 page | turn | 11.16 (11.20) | **9.38** (8.01) |
| | step | 12.80 (10.65) | **10.88** (8.98) |
| `bug1743245.pdf` | turn | 60.07 (47.31) | **43.00** (33.98) |
| | step | 58.60 (45.04) | **39.73** (29.01) |
| `issue14415.pdf` | turn | 11.74 (11.83) | **11.24** (10.27) |
| | step | 6.38 (5.75) | **5.44** (5.64) |

## 4. Measured and dropped

- **Each point's side measured once into a buffer** for both edges it ends, measured with the boxes
  side by side against the arm with neither: +52 / +213 / +11 M; and carried from edge to edge
  without a buffer: +3 / +116 / +2. The early return that reads each side once is the common
  case, and both cost more than they save.
- **The bound raised instead of the tiling** (no tiling, the sweep unbounded): 3 904 / 6 974 /
  2 206 M against 2 087 / 9 535 / 1 393 — cheaper on `bug1743245.pdf`, than the kept arm's 7 105
  too, and 87% and 58% dearer on the others; and every lane that integrates without the set
  takes the tiled pieces (ADR 1407 section 2).
- **The set asked at rim pixels only**: with (b) and (e) the fill asks nothing of a tiled stroke.

## 5. Exactness

Every corpus first page through raster, one encode thread, one rasteriser in sequence, hashed
against the tree before: 1×, 0 of 958 differ; the compute lane at 1×, 0 of 958; 4× on the
page-turn lane, 1 of 957. That page, `issue19971.pdf`, drawn alone at 4×: 6 pixels, one level,
all in one stroke of 1 442 pieces that (e) vouches for. Habit 53: each pixel's area of the pieces,
clipped in `f64` and summed (a 128 × 128 sample finds no point in two), is 0.092163–0.092172, which
is 23.501–23.504 levels; the integral kept reads 24, the set question read 23. **24 is the set's
value** under ADR 0005's rounding, and the move is towards it.

## 6. Left

Against its floor the tiling is still most of the Type 3 page's and `bug1743245.pdf`'s encode: the
separation questions and the cutting are the work, quadratic in the pieces of one tight bend. The
Type 3 page tiles each glyph subpath again at every placement of its cell, which is the encoder's
to cache (ADR 1375 section 2), and a stroke of several subpaths is still asked by the fill, since
the tiling is per subpath.
