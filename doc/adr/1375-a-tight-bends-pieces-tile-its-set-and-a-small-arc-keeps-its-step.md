# 1375 — A tight bend's pieces tile its set, and a small arc keeps its step

Status: accepted. Session 1269. Amends ADR 1361 sections 1 and 3 on raster's side; supersedes
nothing.
Context: ISO 32000-2 §8.4.3.2, §8.4.3.3, §8.4.3.4, §8.7.4.5.3, §10.7.2, §10.7.3, §10.7.4;
`doc/QUORRA_FEEDBACK.md` sections 45, 54 and 55; trap 58; `doc/questions/A76` (raster work is derived
from the clause, not from the other backend).
Code: `raster/crates/raster-gpu/src/raster/stroke.rs` (`stroke_polylines`, `turns`),
`raster/crates/raster-gpu/src/raster/stroke/disjoint.rs`.
Tests: `raster/crates/raster-gpu/src/raster/tests/stroke_set.rs`
(`a_tight_bend_counts_its_rim_once_either_way`, `the_pieces_of_a_tight_bend_tile_its_set`,
`a_round_dot_is_its_disc_within_the_relative_bound`).

**I did not open `crates/render-cpu/`.** Every expected value below is the clause's; the oracle
appears only as a raster read back from the corpus instrument.

## 1. What the hook was paying for, measured before building

ADR 1361 left the pieces overlapping where a curve bends more tightly than the half-width, and
said the hook's rim paid for it, quoting 557.87 at 1× against the set's 565.50. Measured, that premise
does not hold. The area of the pieces' **union**, sampled on a 1/16-pixel grid with no rasteriser,
is 557.83 / 560.73 / 562.38 / 564.30 at 1× / 2× / 4× / 8×, and the ink is 557.87 / 560.76 / 562.39 /
564.30: the gap to 565.50 is the flattened centre line (§10.7.2's chord, carried round an outer rim
twice the curve's length), not a double count. Per pixel, against 64 × 64 samples of the union, the
hook's worst excess was 0.005 at 1× and 0.055 at 2×, in two pixels under the arch where the two
round **caps** cross — pieces of the same subpath, but not neighbours.

The defect is real where a bend is tight and short: a quarter circle of radius 3 at `8 w`, butt
caps, read 39.51 / 38.60 / 39.04 / 38.91 against its union's 38.27 / 38.28 / 38.85 / 38.85, the
worst pixel 0.37 over. There each segment's rectangle reaches past the centre of the bend, and the
rectangles cross one another's edges on the rim.

## 2. The construction: each piece less what the pieces before it hold

§8.4.3.2: "stroking a path shall entail painting all points whose perpendicular distance from the
path in user space is less than or equal to half the line width". That is a set; the fill
integrates winding and clamps it (trap 58), so the pieces must **tile** it wherever they meet on a
rim pixel. ADR 1361's `inner_cut` does that for neighbours while the cut stays within each
segment's half. Extending it along the curve — trimming a piece's inner edge where the next one's
crosses — does not close: past the cut's reach the region a piece loses is not inside its
neighbour (a rectangle's tail beyond the centre is nearest the far end of the bend), and the
partition that would be exact is a straight skeleton with events. A single outline contour with
its inner offset folded is worse: the swallowtail is wound against the body and cancels it, which
is section 54's hole.

What the fill's accumulation does admit exactly is pieces that share no area. Every piece the
stroker emits is convex, and `P − Q` for convex `P`, `Q` is a set of convex fragments with no
boolean library: for `Q`'s edges in order, the part of what is left of `P` outside edge `j` is one
fragment and the part inside is carried on. So where a vertex turns without an inner cut
(`turns`), the pieces that meet it — its segments and join, and every piece of the subpath whose box
meets one of theirs — are re-cut: `disjoint` replaces the `k`-th of them by `P_k − (P_0 ∪ … ∪
P_{k−1})`, the nearest earlier piece first, skipping a pair a line through an edge of either
separates. The union is unchanged, no two fragments overlap, each keeps its piece's winding, and
each crossing point is computed once for both sides of its cut. Every other piece is emitted as
before, to the bit. The work is quadratic in the pieces taking part, so it is bounded — 1 024
pieces, 65 536 fragments — and past either bound the subpath keeps its overlapping pieces, which is
the same set, dearer only in the rim pixels an overlap touches.

**What it costs.** Nothing where no bend is tight. The page it costs most is
`ContentStreamNoCycleType3insideType3.pdf`, glyphs stroked wider than themselves in a tiling cell,
so every bend is tight and each glyph subpath of about 150 pieces is tiled once per tile placement
(about 650 fragments, 1.5 ms each): at 1× about 10 ms → 47–93 ms through raster over three runs of
each arm, against the CPU backend's 58–93 ms on the same runs; at 8× about 184 → 210–320 ms. The
whole corpus's raster total moved within the run-to-run noise of both arms (2.93–3.23 s). A
per-placement cache of the expansion would remove the repetition; that is the encoder's, not this
module's.

Because the whole subpath is tiled, the hook's two crossing caps are tiled too. A stroke that
crosses itself with no tight bend (a thin figure of eight) still overlaps at its crossings, and so
do two subpaths: that is section 45's fill-level ask, which this does not answer.

After: the quarter arc 38.28 / 38.28 / 38.85 / 38.85 (union 38.27 / 38.28 / 38.85 / 38.85), the
same arc with round caps 86.45 / 86.45 / 86.70 / 87.21 (86.48 / 86.46 / 86.70 / 87.22), the hook
557.86 / 560.73 / 562.39 / 564.30; the worst pixel against the union at most 0.006 on all three,
which is the sampling.

**The fixture with a closed form** is an L of arms 2 at `8 w` off the grid, whose set is two
rectangles and Table 54's square, `16 + 16 − 4 + 16 = 44`, and whose coverage in any pixel is
inclusion–exclusion over axis-aligned rectangles. Every pixel at 1×–8×, both ways, is within one
coverage step of it; the pieces' signed areas sum to 44 (miter) and 36 (bevel), all one sign.
Planted away (the tiling off), the sum reads 48 and the 1× ink 44.47.

## 3. A small arc keeps the 0.35 radian step, and why

ADR 1361 section 3 left arcs under 16 device pixels at the fixed 0.35 radian step, 2% of a disc
short. Deriving the step from §10.7.2's distance alone, `r(1 − cos(θ/2)) ≤ 0.25`, is the wrong
direction at a small radius: at `r = 2` it gives `θ ≈ 1.01`, an octagon 10% short of its disc —
§10.7.2 NOTE 2's "not to draw inscribed polygons". The bound that binds there is ADR 0044's
relative one (sixteen chords a turn, 2.55% short at most), and 0.35 is finer than it: eighteen
chords a turn, 2.0% short, at a sag of 0.03 pixels for `r = 2`. So the step stays;
`a_round_dot_is_its_disc_within_the_relative_bound` holds a round-capped dot of `4 w` to its disc
within 2.55% at 1× and 4×, and fails at 11.38 against 12.63 when the 0.35 cap is removed. The
64-gon stays within its closed form at every rung.

## 4. `issue10572.pdf` at 4×: neither side, and not a lane

The page is `61 716 225 -450 re f` under an axial pattern whose function is twelve type 3
stitchings of `/Bounds [.5 .5]` — twenty-four hard stripes, each boundary on a whole device row at
every rung. §8.7.4.5.3 makes a point's colour the function of its own `t`, so every pixel's colour
is determined, and §10.7.3's smoothness bounds an approximation's colour error — a stripe row
displaced is a whole-range error. Against rows computed from the file in exact arithmetic: at 1×
both are right; at 2× the oracle is wrong in 3 rows and raster in none; at 4× 7 and 2; at 8× 13
and 4. The difference is the same on all three of raster's lanes, so it is not the sampled lane's.
Raster's rows are its 4 096-texel ramp read at `round(t · 4095)`, which moves a step by up to half
a texel (0.22 units on this 1 800-unit axis); that is an ask, section 55. The oracle's error grows
along the axis, to four rows at 8×; that is handed to the round owning `render-cpu`.

## 5. What is left

Section 45's fill-level defect (a path whose own contours overlap), and with it the crossings of
strokes that have no tight bend. A butt cap on a flattened curve is square to the last chord rather
than to the curve's tangent, a turn of half the chord's angle: on the quarter arc at 8× its outer
corner is 0.39 units off, the two triangles nearly cancelling in area.
