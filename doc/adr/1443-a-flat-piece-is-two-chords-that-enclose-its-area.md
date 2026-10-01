# 1443 — A flat piece is two chords that enclose its area

Status: accepted. Session 1304. Amends raster's ADR 0008 (the flattening's tolerance, kept) and
ADR 0044 (the relative bound, kept; what it now holds is the polygon's shape, not an inscribed
polygon's shortfall), and ADR 1361's `arc_steps` (kept; its chords now go in pairs). Answers
`doc/QUORRA_FEEDBACK.md` section 59, ask 1. Supersedes nothing.
Context: ISO 32000-2 §8.4.3.2, §10.7.2, §10.7.4; ADR 1435 (the per-pixel reference); habit 53;
traps 1, 13, 58, 62; `doc/questions/A76`.
Code: `raster/crates/raster-gpu/src/raster/flatten.rs` (`flatten_cubic`), `raster/src/compute.rs`
(the compute lane's `flatten_cubic`, statement for statement), `raster/src/raster/stroke.rs`
(`arc_waist`, `arc_fan`, `cap_fan`), `crates/render-raster/tests/corpus.rs`
(`DIFFERS_AT_THE_EDGES`, `frame_digest`).
Tests: `raster/src/raster/tests/flatten.rs` (`a_flattened_curve_encloses_its_own_area`, the circle and
segment-count fixtures), `tests/curve_join.rs` (the round join), `tests/stroke_set.rs` (`within`).

**`crates/render-cpu/` was not opened.** Every expected value below is a closed form from the clauses.

## 1. What the clauses owe a flattening

§10.7.2 fixes what the tolerance *is* — "the maximum permitted distance in device pixels between the
mathematically correct path and an approximation constructed from straight line segments" — lets the
processor choose it ("PDF processors may choose to ignore any flatness tolerance specified within a
PDF file"), and says in NOTE 2 what it is for: "to control the precision of curve rendering, not to
draw inscribed polygons". §10.7.4 names the side an error may fall on: "[t]he area covered by painted
pixels shall always be at least as large as the area of the original shape." A chord between two
points of a curve lies wholly on its concave side, so a flattening of chords on the curve takes about
two thirds of each piece's height times its length out of the shape: the wrong side, on every curve.

## 2. The candidates, measured on circles (closed form first, habit 53)

Radius `r` device pixels, the four-cubic circle, today's test (0.25 px, ADR 0044's relative bound);
error in levels of 255 per unit of rim, against the four cubics' own area by Green's theorem, and
per pixel against exact polygon-in-pixel areas of a 1e-4 flattening. Cost is vertices per turn.

| construction | verts (r ≤ 6 / r = 96) | area / rim, r = 6 | mean pixel error / rim, r = 6 / 24 / 96 |
|---|---|---|---|
| one chord (before) | 16 / 64 | −19.6 | 19.6 / 19.7 / 19.7 |
| one chord at 1/16 px | 32 / 128 | −4.9 | 4.9 / 4.9 / 4.9 |
| one chord at 1/256 px | 128 / 512 | −0.31 | 0.31 / 0.31 / 0.31 |
| tangent polygon (circumscribed) | 32 / 128 | +10.0 | 10.0 / 9.9 / 9.9 |
| control polygon | 48 / 192 | +6.6 | 6.6 / 6.6 / 6.6 |
| apex placed for the exact area | 32 / 128 | 0.000 | 2.0 / 2.6 / 2.9 |
| **midpoint of the inner controls** | 32 / 128 | −0.036 | 2.0 / 2.6 / 2.9 |
| the same, one halving fewer | 16 / 64 | −0.58 | 10.6 / 11.4 / 11.6 |

A stroke of width 1 (the page's rings): per-pixel error per unit of rim at `r` = 4, one chord 24.0,
the chosen construction 2.0, one chord at 1/256 0.38.

**Built: each flat piece is two chords through the midpoint of its two inner control points.** From
the piece's start, the area between a cubic and its chord is `(3/20)·(p1×p2 + p1×p3 + 2·p2×p3)`;
with the controls at `c/3 + a·n` and `2c/3 + b·n` that is `−(a + b)·|c|/4`, which is exactly the
triangle through `(p1 + p2)/2`. Elsewhere it misses by `|c|·((3α − 1)·b − (3β − 2)·a)/20`, second
order, shrinking with every halving. It is the exact-area apex to a hundredth of a level at no
division — a halving of a sum, the same bits on every adapter, which the compute lane mirrors
statement for statement. The circumscribed candidates satisfy "at least as large" literally and put
five to ten levels on every rim pixel; the residual here is under a tenth of the byte's own half-level
rounding, and it is the construction the per-pixel reference pays for. The one-halving-fewer variant
keeps the vertex count and was rejected: it holds the §10.7.2 distance but leaves 11 levels a rim
pixel. The polygon stays within the tolerance of the curve on either side (the midpoint is within it
of the chord, as the curve is), so the stroke fixtures' allowance became two-sided.

**Round joins and caps take the same construction** (`arc_waist`): `arc_steps`' chords are taken in
pairs, each pair a step of angle `a` drawn through the point at radius `r·(a/2)/sin(a/2)` on its
bisector, which encloses the step's own sector — at the vertex count the arcs had. The round-join
fixture now holds the quarter disc to rounding on both sides.

## 3. What moved

`issue2177.pdf` against ADR 1435's reference, rebuilt from the page's geometry (128 × 128 samples a
pixel; it reproduces 1300's figures within 0.07): raster **2.00 → 0.197** of 255 (oracle 0.644), ink
12933.9 → 13013.6 against 13009.6 (oracle 13030.3). Against the oracle 1.9385 / 11.54 → 0.4930 /
6.42: the page agrees and leaves `DIFFERS_AT_THE_EDGES`, which is empty. The flattening alone, with
the product kept at the residue, takes the page to 0.187 and the worst tile (32, 224) from 7.42 to
1.09 — so the flattening was that tile's cause too (ADR 1444 for the residue).

Corpus at 1×, frame digests of this build against HEAD (`PDFVIEWER_RASTER_TIMES` now writes each
page's digest and mean): 645 of 961 pages moved; 599 toward the oracle, 33 with an equal mean, 13 away,
the sum of their means 79.6 → 47.5. The thirteen are text pages — `pr12564` 0.21 → 0.44, the six
`tracemonkey` pages 0.20 → 0.35, the rest under 0.03 — where the glyph outlines now enclose their own
area; the ink ladder names the side (below). Gate: 961 agree, 0 differ, 6 refused, 7 not
comparable; thread count 0. At 4×: 641 moved, 635 toward, 6 equal, none away (sum 38.9 → 10.5); 958
agree. The GPU lane's `bug1743245.pdf` (2.6131 at HEAD, 2.6102 here) and the compute lane's
`issue1905.pdf` refusal are HEAD's, each measured on both trees.

The thirteen are moves toward the set, read with `examples/ink_ladder` (habit 53's question asked of
a page): `pr12564` at 1× held 60238 where its ink converges on about 60420 by 8×, and holds 60424;
`tracemonkey_a11y` 28474 → 28639 against about 28649. The oracle reads 60408 and 28626 — light by the
inscribed chord this section describes, which is its own lane's question and not this round's.

## 4. Cost, and the lever that was not taken

The flattening places twice the vertices on a curve; lines are unchanged. Callgrind, one
`examples/frame_budget` round (turn, warm, step), whole process, HEAD against HEAD plus this round's
raster files only (separate trees and target directories, trap 50), M instructions:

| page | HEAD | built | one-sided stroke test ×4 (rejected) |
|---|---|---|---|
| ISO 32000-2 page 101 (text) | 862 | 913 (+5.9%) | 913 |
| `personwithdog.pdf` | 934 | 937 | 934 |
| `issue14415.pdf` (curved strokes, tight bends) | 1 418 | 2 126 (+50%) | 1 473 |
| `bug1743245.pdf` | 7 100 | 7 213 (+1.6%) | 7 216 |
| `issue2177.pdf` | 783 | 894 | 881 |

Two costs were removed on the way, each measured: ADR 1444's sliver test took a square root per edge
of every fragment (`bug1743245.pdf` +14%) and now uses the box; and two chords per arc step cost the
same page +7%, which pairing the steps removes. **`issue14415.pdf`'s +50% is kept, and the lever is
named**: its strokes bend more tightly than their half-width, and the tiling of a tight bend is
quadratic in its pieces (ADR 1375). Testing a stroke's pieces one halving coarser — the two chords then
stand about as far from the curve as one chord did, at the old vertex count — takes it to +3.9%, but
on `inks.pdf` at 4× it drew a solid wedge the stroke's set does not hold (worst tile 12.35, a piece
21 px from a centreline whose half-width is 10), at a hairpin where a 0.8-px chord meets a 2.8-px one
under round joins; with the fill's test the corpus is clean. That is a stroker defect at a hairpin
the coarser flattening reaches, and it is left with its reproduction (`doc/history/1304-…`): the
lever is available once the stroker's hairpin is answered. The launch-path gate passed.
