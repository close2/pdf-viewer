# 1389 — A pixel is covered by its set, a butt cap is square to its tangent, and a step lies at its own `t`

Status: accepted. Session 1276. Amends ADR 1361 section 1 and ADR 1375 section 2 on raster's side
(the fill no longer asks the stroker for a tiling to be exact); amends ADR 0011 on where a ramp's
texel is chosen; supersedes nothing.
Context: ISO 32000-2 §8.5.3.3.2, §8.5.3.3.3, §10.7.4, §8.4.3.3, §8.7.4.5.3, §7.10.4, §10.7.3;
`doc/QUORRA_FEEDBACK.md` sections 45 and 55; traps 54, 56, 58, 62; `doc/questions/A76`.
Code: `raster/crates/raster-gpu/src/raster/fill.rs`, `fill/topology.rs`, `fill/overlap.rs`,
`fill/exact.rs`, `raster/flatten.rs` (`Ends`), `raster/stroke.rs` (`open_ends`, `carried`,
`segment_piece`), `device/ramp.rs`, `shaders/shading.wgsl` (`ramp_texel`), `resources.rs`
(`winds_two_values`), `encode/fill.rs`, `encode/parallel.rs`.
Tests: `raster/tests/fill_set.rs`, `raster/tests/stroke.rs`
(`a_butt_cap_on_a_curve_is_square_to_its_tangent`), `device/ramp/tests.rs`,
`tests/shading_steps.rs`, `tests/compute_lane.rs` (unchanged, and holding).

**I did not open `crates/render-cpu/`.** Every expected value below is derived from the clauses in
closed form; the oracle appears only as the corpus gate's and `ink_ladder`'s readings.

## 1. The fill: a pixel's coverage is the area of the inside set

§8.5.3.3.2 decides insideness per point — "if the result is 0, the point is outside the path;
otherwise, it is inside" — and §8.5.3.3.3 by parity; §10.7.4 scan-converts after "all 'insideness'
computations have been performed". So a pixel holds the area of the inside set, and the clamped
integral of the winding equals it exactly when the winding inside the pixel takes at most two
neighbouring values (the rule is linear between two neighbouring integers, for both rules).

**The construction, in three tiers, cheapest first.** (a) A fill that is a few convex subpaths,
one orientation, boxes apart, winds two values everywhere (`plainly_two_values`). (b) Otherwise
every pair of edges whose boxes meet is tested once, by a sweep along the longer axis: a subpath
crossing itself, or two crossing each other, is found; where there is none, a subpath adds `0`
outside and its orientation inside, insides nest or stand apart, and if each subpath's orientation
turned over once per subpath holding it is the same everywhere, the fill winds only `0` and one
value (`Topology::two_values`) — a glyph's contours and a stroke's abutting pieces. (c) Otherwise
the edges are walked once more into per-row spans, and each pixel asks the same nesting rule of
the subpaths through it; a pixel that fails is recomputed by `exact`: the row cut into bands at
every end point and crossing of the pixel's own edges; the winding along a vertical line through
the pixel that no vertex lies on known up to one integer; that integer read off the integral
itself (`K = round(average − relative average)`, declined if not within 0.25 of a whole number);
the inside intervals deposited as trapezoids into a one-pixel window. Only complex pixels are
written; every other pixel keeps its integral to the bit.

**A per-row set pass was built first and measured**: it flagged 670 k of 1.3 M rows on the corpus
(every glyph stem shares pixels with its own other side) and cost 11.8 s of thread time; the
pixel-local construction with tier (b) takes 130 k pixels and 0.4 s. A planar-map decomposition
was not built: tier (c) is it, confined to the pixels that need it.

**Bounds, each stated and each leaving the integral** (what every pixel had before): the sweep
stops past 8 comparisons per edge-or-pixel of the fill (a few pixels crowded with thousands of
pieces); `2^20` spans; 8 subpaths through one pixel; 32 edges through one complex pixel. A crossing
of two subpaths exactly at a corner both share is not seen (abutting pieces share corners to the
bit and must not count); that is the one blind spot.

**The lanes.** A stored outline answers the two-values question once for all its placements
(crossing and nesting are what an invertible transform keeps): the CPU lane's jobs skip the
question for an outline that answered yes; the compute lane, and ADR 0090's hybrid, take only such
outlines — anything else is drawn by the CPU lane, so `tests/compute_lane.rs`'s byte identity
holds (it failed on its self-crossing stars before this routing: 25 pixels, max 64, on RADV).

**Closed forms** (`fill_set.rs`, every pixel, 1×/2×/4×/8×, max deviation in coverage steps, before →
after): section 45's band stated twice 63.75 / 127.5 / 0 / 0 → ≤ 0.5; two overlapping squares, four
cases (same way non-zero 46.4 → 0.5, even-odd 92.3 → 0.5, opposed 92.3 → 0.5); a five-point star
non-zero 42–64 → 0.50, even-odd 50–129 → 0.50; two apart squares wound against each other 153 →
≤ 0.5; nested same-wound squares ≤ 0.5 both rules; a tile of the star equal to the whole's crop.
Planting the parity rule away fails the apart and nested cases.

**Cost** (A/B, same build, machine quiet): corpus at 1× through raster 3.75 / 3.77 s → 4.00 / 3.96 s,
median page ratio 1.30 → 1.38–1.39; at 4× on the CPU lane 19.06 → 19.29 s, median 2.40 → 2.49.

**Pages.** `issue20232.pdf` 17 926.55 / 17 893.11 / 17 867.14 / 17 861.98 against the oracle's
17 932.18 / 17 900.06 / 17 873.66 / 17 866.07 (1×–8×); `issue15150.pdf` 0.17 at every rung;
`issue21068.pdf` equal at every rung. The corpus gate reads 959 agree / 2 differ at 1×, and 958 /
0 at 4× on both the CPU and the GPU lane.

## 2. The butt cap is square to the curve's tangent

§8.4.3.3, Table 53: "The stroke shall be squared off at the endpoint of the path. There shall be no
projection beyond the end of the path." The path's direction at its endpoint is the Bézier's
derivative there, which flattening discards; `flatten` now keeps it (`Ends`: the first non-zero of
`c1 − p0`, `c2 − p0`, `p3 − p0` leaving, and the mirror arriving), and the stroker turns the end
piece's end edge and the cap to it wherever the end chord can carry it (`hw · tan θ ≤ |chord|`,
`cos θ > 0`; otherwise the chord's square end stands). Quarter arc of radius 20 at `4 w`: the end
projected 0.20 / 0.10 / 0.10 / 0.05 units past `x = 50` → 0; radius 3 at `8 w`: the outer corner
0.80 / 0.80 / 0.40 / 0.40 units from the closed form → 0 (its inner corners lie inside the bend's
tiling). Joins between a curve and a line are still square to the chords.

## 3. A ramp's hard step lies at its own `t`

§8.7.4.5.3 gives a point the function at its own `t` (Table 79: "The variable t becomes the input
argument to the colour function(s)"), and §7.10.4 gives a bound to the interval that starts there.
The ramp is now cut at every coincident stop pair into segments, each sampled on its own grid in
row 0, with the bounds as exact `f32` bits in row 1 and each segment's first texel and count in
row 2 (`RAMP_ROWS = 3`, 48 KiB a ramp). The shader finds the segment by comparing `t` with the
bounds (closed on the left; a `t` at or below the first offset is the first segment's) and rounds
only within it. A smooth ramp is one segment and its texels and lookup are unchanged to the bit.
Past 1 024 segments a ramp is one segment and rounds, as before. `ramp.rs`'s comment called the old
snap one-sided; the reader rounded, so it moved a step up to half a texel either way.

`issue10572.pdf`'s stripes (`shading_steps.rs`, the page's axis and 24 stripes in device space,
rows against exact arithmetic): 0 / 0 / 3 / 5 wrong rows at 1×/2×/4×/8× → 0 at every scale.
The brief's §8.7.4.5.2 is the function-based shading; `/Extend` and `/Domain` are Table 79's.

## 4. Left

The encoder's per-placement cache for re-tiled Type 3 glyphs was not measured. `issue1905.pdf`
left the 1× device-refusal list during this round (it agrees); nothing here charges the frame's
scene-byte budget differently, and the list is updated because the gate is this round's.
