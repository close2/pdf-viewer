# 1455 — A point inside a curve is no corner, and is joined by the set's disc

Status: accepted. Session 1310. Answers ADR 1443 section 4's "a stroker defect at a hairpin the
coarser flattening reaches" and `doc/history/1304-…`'s "Left: the stroker's hairpin". Amends ADR 1361
and ADR 1397 in one respect: the join drawn at a point a curve's flattening adds is round, whatever
join the stroke names. Supersedes nothing.
Context: ISO 32000-2 §8.4.3.2, §8.4.3.4, §8.4.3.5, §10.7.2; habits 50 and 53; traps 1, 13, 58;
`doc/questions/A76`.
Code: `raster/crates/raster-gpu/src/raster/stroke/centre.rs` (`Centre::of`, `Centre::inside_a_curve`),
`raster/crates/raster-gpu/src/raster/stroke.rs` (`stroke_subpath`'s joins).
Tests: `raster/crates/raster-gpu/src/raster/tests/inside_a_curve.rs`.

**`crates/render-cpu/` was not opened.** Every expected value is the set's, sampled from its predicate.

## 1. The piece

Reproduced with ADR 1443's coarser stroke test (the flat test four times looser) on `inks.pdf` at 4× by printing every piece whose edges
reach past `hw + 0.75` of the stroke's own chords: one piece, the quadrilateral
`(1170.15, −2257.04) (1162.20, −2263.10) (1150.80, −2248.15) (1169.58, −2247.06)` — vertex, outer corner,
tip, outer corner — reaching 21.3 px from the chords at half-width 10. That is **a miter join**, not a
round one: the stroke joins by miter (the history file's "round caps and joins" was wrong about the
join), and the vertex is the third-last point of one cubic's flattening, which turns 124° between a
0.8-px chord and a 2.8-px one. The miter is the clause's own construction there — at an angle `φ ≈ 56°`
between the segments the ratio is `1 / sin(φ/2) ≈ 2.1`, and §8.4.3.5's "When the limit is exceeded,
the join is converted from a miter to a bevel" does not fire under the default limit of 10 (its EXAMPLE:
"a limit of 10.0 converts them for j less than approximately 11.5 degrees"). So the brief's reading — a
miter the limit should have cut, or a folding inner side — was not the piece; the tip is exactly where a
miter between those two chords belongs. What was wrong is that there is a join there at all.

## 2. The clause

§8.4.3.4: "Join styles shall be significant only at points where consecutive segments of a path
connect at an angle". A curve is one segment; the points its flattening adds are where the chords that
stand for it meet, and no two segments of the path do. What a stroke paints there is §8.4.3.2's set,
"all points whose perpendicular distance from the path in user space is less than or equal to half the
line width", whose closed form round a point between two chords is the union of the two chords'
rectangles and the disc of the half-width about the point — a round join. A miter there stands
`hw · (sec(θ/2) − 1)` past the set, invisible at a gentle turn and a wedge at a hairpin; a bevel there
leaves the disc's segment out. **Built: `Centre` records which of its points lie inside one curve**
(after a curve's leaving direction and before its arriving one, both of which `flatten_stroke` already
records, ADR 1397), **and the join there is round.** Every other point — where two segments meet,
curve or line — keeps the stroke's join. A round join at a gentle turn is one arc step, four points, as
a miter is. A turn between two curves that meet with equal tangents is still joined by the stroke's
style between their chords; it is not inside a curve, and its turn is the flattening's alone.

## 3. Fixture and corpus

`inside_a_curve.rs` holds the reproduction's tail, moved near the origin, as one curve and as three
lines, mitered at limit 10 with round caps so that the set is the chords' distance set. As one curve:
no ink on a pixel wholly beyond `hw` plus §10.7.2's tolerance, every pixel within 1.5 × tolerance ×
255 of the set's share on a 32 × 32 sample grid (worst 31.9, on the caps' and the join's arc rims), and
the ink 99155 against the set's 99161. Before the change it failed on 13 pixels of the wedge (up to
255 where the set is 0) — watched (trap 13). As three lines the hairpin is a corner and the miter
stands past the set, as Table 54 says it must.

Corpus at 1×, frame digests with and without the change in one binary: 36 of 961 pages moved; against
the oracle 4 closer, 31 equal to four places, 1 further (`rotated_ink.pdf` 0.0351 → 0.0352). The one is
held to the geometry, not the oracle (habit 50): its ink ladder reads 229.84 → 229.79 at 1×, 228.77 →
228.74 at 2×, 228.72 → 228.69 at 4×, against 228.70 → 228.67 at 8× — closer to the converged ink at
every rung. At 4×: 45 of 958 moved, 4 closer, 41 equal, none further. Gates 961/0/6/7 at 1× and 958/0/8/8 at 4×, one-vs-many 0 on both.

## 4. The coarser flattening ADR 1443 declined, re-measured

With the hairpin answered, `inks.pdf` at 4× agrees under the coarser stroke test as well, so the
defect no longer decides it; the closed form does. Rings stroked over the four-cubic circle, against
the annulus's share of each rim pixel (32 × 32 samples), mean |error| per rim pixel, finer → coarser:
`r` 6 `w` 2: 1.69 → 8.70; `r` 24 `w` 2: 2.09 → 8.87; `r` 24 `w` 8: 2.04 → 8.97; `r` 96 `w` 2: 3.38 →
8.69; `r` 48 `w` 30: 1.77 → 4.57; worst pixel 15.6 → 45.2. Every rim moves away from the set by four
times its error, so the brief's condition — keep it if no pixel moves away from the set — refuses it,
and `issue14415.pdf`'s +50% stays with its lever named as before: the tiling of a tight bend
(ADR 1375), not the flattening.
