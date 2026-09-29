# 1397 — A join is square to its curve's tangent, and the fill set is asked only where it can be answered

Status: accepted. Session 1280. Amends ADR 1389 sections 1 and 2 on raster's side (the question's
cost, and the join a butt cap's tangent left square to the chord); amends ADR 1361 section 1 (a
piece's inner cut is the crossing of its own edges); supersedes nothing.
Context: ISO 32000-2 §8.4.3.2, §8.4.3.3, §8.4.3.4, §8.4.3.5, §8.5.3.3, §10.7.2, §10.7.4;
`doc/QUORRA_FEEDBACK.md` section 56; traps 50, 54, 56, 58, 62, 65; habits 45, 47, 52;
`doc/questions/A76`.
Code: `raster/crates/raster-gpu/src/raster/fill/topology.rs` (`shape_of`, `contains`,
`meeting_pairs`, `ordered_bits`), `raster/flatten.rs` (`Tangent`, `flatten_stroke`, `convex`),
`raster/stroke.rs` (`stroke_pieces`, `Stroked`, `stroke_subpath`, `join_at`),
`raster/stroke/centre.rs` (`Centre`, `turned_ends`, `inner_cut`), `encode/parallel.rs`
(`rasterise`), `encode/stroke.rs` (one call), `raster-pages/src/page.rs` (two rows).
Tests: `raster/tests/curve_join.rs` (new), `raster/tests/fill_set.rs`
(`a_hairline_far_from_the_origin_winds_two_values`), `raster/tests/stroke_set.rs` and
`raster/tests/stroke.rs` (unchanged statements, flattened for a stroke), `tests/archetypes.rs`.

**I did not open `crates/render-cpu/`.** Every expected value is derived from the clauses in closed
form; the oracle appears only as the corpus gate's reading.

## 1. Where the fill set's price went

Instruction counts, callgrind, one thread, each arm its own export and target directory with
`md5sum`-distinct binaries (trap 50), `examples/r1280_page` (a scratch harness: one page through
`QuorraRasterizer::render`, three frames). Wall clocks could not resolve it: the oracle's own time
moved 2.52–3.20 s between runs of identical code. The pages are those the construction cost most
at 1×, plus ISO 32000-2's page 101. "tiles + joins" is that lever alone, measured on the join
construction before section 2's smooth-joint rule and `flatten_stroke`.

| M Ir, 3 frames | HEAD | f64 point | sort | tiles + joins | final | no per-region question |
|---|---|---|---|---|---|---|
| `issue12810.pdf` | 12 491 | 7 557 | 11 911 | 4 892 | 4 644 | 3 599 |
| `bug1743245.pdf` | 9 790 | 9 786 | 8 953 | 9 790 | 8 980 | 7 295 |
| `issue14415.pdf` | 1 078 | 1 079 | 983 | 955 | 964 | 650 |
| `issue20513.pdf` | 429 | 429 | 407 | 430 | 406 | 237 |
| `issue9418.pdf` | 2 564 | 2 549 | 2 528 | 2 622 | 2 577 | 2 253 |
| page 101 | 158.0 | 158.7 | 157.1 | 164.4 | 152.6 | 159.0 |

Over the forty pages the construction costs most (one frame each): **13 244 → 10 131 M**, the
construction itself (against the arm with no per-region question, habit 52's "off" arm, 8 607 M)
**4 637 → 1 524 M**. The per-function profile on `issue12810.pdf` (a map of hatch lines) was
`Topology::of` 22%, the pixel walk inlined in `fill_mask_settled` 40%, the sweep's sort 11%.

**Lever (b), kept: a hairline's cap was read as nested.** `shape_of` stepped `1e-3` of a subpath's
longest edge inward in `f32`; for a `0.12`-wide round cap at a coordinate of a thousand that is
`6e-5`, half the `f32` spacing, so the point landed on the edge the body shares and the body "held"
the cap. 29 042 region walks a frame on `issue12810.pdf` → 480; the point and the ray test are now
`f64`. `a_hairline_far_from_the_origin_winds_two_values` reads `Some(false)` on HEAD.

**Lever (c), kept: the sort, not the bound.** The sweep already runs along the longer axis; the
indirect `total_cmp` sort was the cost. Boxes are sorted as packed `(key, index)` integers:
−8.5% on `bug1743245.pdf`, −9% on `issue14415.pdf`. The answer does not depend on the order of
boxes with one start (every meeting pair is visited, the comparisons counted the same). The bound
itself is spent on `bug1743245.pdf`'s strokes of ~950 pieces in under 75 pixels, 205 a frame.

**Kept: a stroke that tiles by construction.** `stroke_pieces` returns `tiles` where the stroke is
one subpath that is one straight segment (body and caps share corners to the bit), or closed and
convex with every corner cut (a strip per edge and a sector per corner); such a stroke keeps its
integral unasked. `a_stroke_whose_pieces_tile_draws_the_same_without_the_question` holds seven such
shapes within a byte of the asked fill at 1×–8×; four that may overlap are not vouched for.

**Measured and dropped:** skipping the pair tests inside a convex subpath (+0.1% to +1.3% alone).

**Lever (a), answered.** A stored outline's answer is kept across frames and placements: page 101's
three frames cost the same with and without the per-region question. The Type 3 page
(`ContentStreamNoCycleType3insideType3.pdf`) asks the same 96 regions every frame (24 past the
bound, 51 two values, 20 walked, 1 plain): fills the walk draws inline, or residue-clipped, which
never consult the outline (ADR 1395). A completed "yes" carries exactly to every region of the
fill; a bound-hit answer does not, so carrying it needs a three-valued cache — not built, it is in
the encoder's walk path; at most 177 M of that page's 1 515 M.

**The largest lever left is ADR 1375's tiling.** With it off, the forty pages read 6 812 M (below
the no-question arm's 8 607, which still tiles), `bug1743245.pdf` 2 999 → 405 M, the Type 3 page
501 → 99 M, the corpus at 1× still 959 / 2 with the same two pages, and every per-pixel fixture
passes; `the_pieces_of_a_tight_bend_tile_its_set` fails by design. The fill is exact on overlaps
only within its bounds (8 subpaths, 32 edges a pixel), so dropping the tiling is a decision to
measure those bounds on tight bends first. Not kept: the brief keeps a lever only with every raster
test unchanged.

## 2. The join where a curve meets another segment

§8.4.3.4: "Join styles shall be significant only at points where consecutive segments of a path
connect at an angle". Table 54 builds each join from the two segments' own strokes: "The outer edges
of the strokes for the two segments shall be extended until they meet at an angle"; "An arc of a
circle with a diameter equal to the line width shall be drawn around the point where the two
segments meet, connecting the outer edges of the strokes for the two segments"; "The two segments
shall be finished with butt caps". A curve's stroke at its end runs along its tangent.

`flatten_stroke` keeps a `Tangent` at every point where a curve begins or ends; `Centre` carries
them through the stroker's deduplication; each piece's end is turned to the tangent where the chord
can carry it (`hw · tan θ ≤ share · |chord|`, half each where both ends of one chord turn), and the
join, the miter limit and the inner cut take the two directions the pieces end square to. The inner
cut is the crossing of the two pieces' own inner edges (their offset corners), within the half of
each nearest the vertex; between pieces square to their chords it is ADR 1361's `hw · tan(θ/2)`, to
the bit. A point where the tangents turn no more than the chords either side of it is no corner the
flattening can tell from its own and is drawn as every flattened vertex is — without that rule a
tangent-continuous joint read a noise-signed turn, declined its cut, and sent `inks.pdf` 8% dearer.
A fill never reads tangents, so `flatten` keeps none: page 101 158.0 → 152.6 M.

**Closed forms** (`curve_join.rs`, a quarter circle of radius 30 arriving straight down at `(50, 50)`
and a line leaving along `x`, `8 w`, both directions, 1×/2×/4×/8×): beyond both ends the miter is
the square `(46, 50)–(50, 54)` and the bevel the triangle `(50, 50), (46, 50), (50, 54)`, clipped to
every pixel in `f64` — worst pixel ≤ 1 step; planted back to the chord, 85 / 94 / 194 / 200 steps.
The round join's quadrant holds `4π` less at most its arc's flattening strip, no ink beyond radius 4.

**The hook** (`20 20 m 20 32 32 32 32 20 c`, `16 w`): 557.686 / 560.729 / 562.384 / 564.297 against
565.50, unchanged to the third decimal — it is one cubic, it has no join between a curve and
another segment, and its gap is the centre line's flattening (ADR 1375's reading stands).

**Archetypes** (`raster-pages`): artwork 66 → 67 regions, 384 → 380 per-tile rasterisations,
3 542 360 → 3 555 182 texels; drawing 245 → 252 texels — the strokes' pieces end square to each
cubic's tangent where two cubics meet, which moves their boxes. Recorded with that reason.

## 3. Gates

Corpus 1×: 959 agree / 2 differ (`issue19083`, `issue2177`), exit 0. 4×: CPU lane 958 / 0, GPU lane
957 / 0, compute lane 958 / 0, each exit 0. `tests/compute_lane.rs` on every adapter including
RADV (trap 65). `raster_golden` untouched: the CPU oracle did not change.
