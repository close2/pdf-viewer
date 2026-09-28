# 1361 — A stroke's pieces are wound one way and meet edge to edge, so they add up to its set

Status: accepted. Session 1262. The graphics backend's side of ADR 1348 section 1, built from the
clause alone.
Context: ISO 32000-2 §8.4.3.2, §8.4.3.3, §8.4.3.4, §8.4.3.5, §8.4.3.6, §10.7.2;
`doc/QUORRA_FEEDBACK.md` sections 45 and 54; `doc/questions/A76` (raster work is derived from the
clause, not from the other backend).
Code: `raster/crates/raster-gpu/src/raster/stroke.rs` (`join_at`, `inner_cut`, `segment_piece`,
`held_by_a_neighbour`, `cap_fan`, `arc_steps`).
Tests: `raster/crates/raster-gpu/src/raster/tests/stroke_set.rs`.

## 1. The set, and the instrument it is filled by

§8.4.3.2: "stroking a path shall entail painting all points whose perpendicular distance from the
path in user space is less than or equal to half the line width". Table 54 adds, at a corner, the
miter ("The outer edges of the strokes for the two segments shall be extended until they meet at an
angle, as in a picture frame"), the round join ("An arc of a circle with a diameter equal to the
line width shall be drawn around the point where the two segments meet, connecting the outer edges
of the strokes for the two segments") or the bevel's triangle; §8.4.3.5: "When the limit is
exceeded, the join is converted from a miter to a bevel."

`stroke_polylines` states that set as convex pieces — a rectangle per segment, a join per vertex, a
cap per open end — and `fill` rasterises them under the non-zero rule by integrating the signed
winding over each pixel and clamping afterwards. Two consequences decide the construction:

1. **A piece wound against the others subtracts.** Where it overlaps a piece it cancels it; where
   the two share a pixel the pixel reads `|a − b|`. `join_at` built its wedge `v, p1, p2` with the
   side flipped by the sign of the turn, which mirrored the wedge too: on a turn with `cross > 0`
   every join was wound against the rectangles. A thick curve's joins reach across their
   neighbours, so a `16 w` hook drew radial slivers one way round and solid the other.
   **Rule: every piece has the rectangles' orientation on both turns** — the join visits `p2`
   before `p1` when `cross > 0`, which undoes the mirror, and `arc_fan` follows its endpoints.
2. **An overlap is free inside the stroke and counted twice at its rim.** A rim pixel holding `+a`
   from one piece and `+b` from an overlapping one reads `a + b`, not their union. On a thin ring
   every pixel is rim: a 64-gon of radius 30 at `1 w` read 189.13 against its closed form 188.42.
   **Rule: pieces meet edge to edge wherever they can.** At a join the two rectangles are cut
   along the line from the vertex to the point where their inner edges cross, `hw · tan(θ/2)`
   back along each (`inner_cut`); the part each loses lies inside the other, so the union is
   unchanged, and the outer side's join starts on the rectangles' own end points to the bit. The
   cut is taken only while that distance is at most half of each segment — past it the cuts at a
   segment's two ends could meet, which is exactly a curve bending more tightly than the
   half-width — and there the pieces overlap as before: the same set, dearer only at the rim.

## 2. A path that turns straight back

An exact reversal runs the second segment along the first one's line, so the shorter rectangle is
a subset of the longer: it is not emitted (`held_by_a_neighbour`; of two equal ones the lower index
goes, which leaves the last of any chain standing). That is the out-and-back outline `s` makes of a
two-point subpath, which section 45 named as the stroker's instance of its defect. §8.4.3.4's round
join is significant there — the segments "connect at an angle" of a half turn — and its arc is a
half disc round the vertex: `cap_fan`, which is built for exactly pi. A miter at a reversal is past
every limit, and a reversal's bevel has no area.

## 3. Arcs to the flatness tolerance

A round join's or cap's arc was cut at a fixed 0.35 radian, whose sag grows with the radius: a
64-pixel join fell a whole pixel inside its arc. `arc_steps` now takes `sqrt(8 · FLATTEN_TOLERANCE /
r)`, which bounds the sag by the tolerance every curve here meets (§10.7.2), capped at 0.35 (so every
arc under 16 device pixels is cut as before) and at 256 steps per arc, because a width is a
document's number and a step count an allocation; past about 13 000 pixels an arc is coarser than
the tolerance. `cap_fan`'s two corners are the rectangle's own `end ± n`, not recomputed through
`cos` and `sin`.

## 4. Fixtures, against the set and not against a backend

Each drawn both ways at 1×, 2×, 4×, 8× (the caller's `ink_ladder` as a gate); the two directions
must agree within a sixteenth at every pixel. Ink is scale-normalised, 1× → 8×:

| fixture | the set | raster |
|---|---|---|
| hook `20 20 m 20 32 32 32 32 20 c`, `16 w`, `1 J 1 j` (sampled, 1/64 grid) | 565.50 | 557.87 → 564.30 |
| circle radius 30, `1 w`, four cubics (tube formula, sampled length) | 188.52 | 188.17 → 188.50 |
| 64-gon radius 30, `1 w`, round joins (closed form) | 188.42 | 188.36, 188.41, 188.42, 188.42 |
| L of 40 + 40 at `8 w`, miter / round / bevel | 640 / 636.57 / 632 | 640.16 → 640.00 / 636.51 → 636.47 / 632.16 → 632.01 |
| §8.4.3.5's example, limit 1.414 and 1.415 at 90° | 632 and 640 | as the bevel and the miter |
| reversal, `8 w`, round / miter | 345.13 / 320 | 344.64 → 344.89 / 320.00 |
| `[25 10] 0 d` on an L, `6 w`, round caps and join, dashes as the caller hands them | 442.89 | 441.11 → 441.66 |

A shortfall is bounded by the strip a chord within the tolerance may cut off (the rim's or the
arcs' length times `FLATTEN_TOLERANCE / s`); an excess by the byte rounding of partial pixels. The
per-pixel check on the hook (inside by `hw − √2/2 − 1/4` is full, outside by `hw + √2/2` is empty)
fails the slivers; `the_pieces_of_a_thin_stroke_tile_its_set` sums the pieces' signed areas with no
rasteriser and must meet 640, 632, the 64-gon's `P·w` and the out-and-back's 120 exactly, all of one
sign. Planted away: the winding fails five fixtures (the two directions 25 to 253 of 255 apart at a pixel), the
cuts fail the tiling sum (656 against 640), the reversal skip fails it (240 against 120), the arc
steps fail the hook and the reversal at 8×.

## 5. The cross-backend run

`render-raster --test corpus`: 946 agree / 5 differ before, **949 / 2** after.
`ContentStreamNoCycleType3insideType3.pdf` reads 5117.46 against the oracle's 5121.42 at 1× and
5094.44 against 5094.61 at 8×: onto the oracle, and nothing stands behind it at the gate's
tolerance — §8.7.3.1's unspecified tile order where its cells overlap paints the same colour either
way here. `issue20232.pdf` (+32.3% at 1× to +4.4% at 8×) now reads 17 926.58 against 17 932.18 and
`issue15150.pdf` 0.17 against 0.17 at every rung: both were out-and-back subpaths. `issue19083.pdf`
(section 24c, clip product) and `issue2177.pdf` (edges) stay, their numbers refreshed.

**I did not open `crates/render-cpu/`.** The construction is the clause's; ADR 1348 was read as the
statement of what the oracle does, not its code, and the corpus run is where the two met.

## 6. What is left

Section 45's fill-level defect — a filled path whose own contours overlap is clamped as an integral
— is not a stroker question and stands. Where a curve bends more tightly than the half-width the
pieces still overlap, and the hook's rim pays for it. Arcs under 16 device pixels keep the 0.35
radian step, within the tolerance but 2% of a disc short.
