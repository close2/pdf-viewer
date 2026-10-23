# 2269 — The joins the pieces lost were three roundings, and none was a chord's bound

Status: accepted for integration. Session 1717, 2026-10-23, the pixels slot of batch one hundred and
ten. Clause: ISO 32000-2 §8.4.3.2, §8.4.3.4 and §10.7.4; the §10.7.4 row stays `departed` for its
other departures. Context: ADRs 1348, 1359, 2252 §4 and §5, 2268; trap 236.

## 1. What was lost

ADR 2252 §5 left open a round join or cap the pieces lose at some chords and scales:
`issue14438.pdf` at `(1594, 1093)` — holes at a sixty-fourth at 4x, the join drawn at a twentieth
at 4x and lost again at 64x — and `inks.pdf`'s red ink, which ended in a notch at a sixty-fourth.
§8.4.3.2's stroke holds every point within the half-width of the path, so the disk about a vertex
is in it whatever the join; a probe that asks the pieces, in `f64`, whether they cover points of
that disk found three faults, each drawn at its pixel at two flatnesses and two scales (trap 236):

- **A curve of no length.** The ink states its corner at `(399.299, 518.497)` and then states it
  twice more as cubics whose every control point is that corner. A Bézier's points are weighted
  sums whose weights sum to one only in exact arithmetic, so each such curve's chords were a few
  roundings long and pointed anywhere, and the joins at both ends of them were turned from those
  directions: the outer side of the corner was left out at scales 1, 4 and 64 and drawn at 16. The
  flattening now skips a curve every control point of which is its start (`states_no_length`):
  it adds nothing to the set and states no direction.
- **A round join turned by a rounding.** At a chord turn of nearly a half turn, where the red ink
  doubles back inside one curve, the arc's direction was the sign of `near × far`, two nearly
  opposite normals: at some chords it put the arc behind the vertex and left the notch. The arc
  turns from `near` towards the way the path arrives, `near × (at − from)`, which is the same
  answer at every turn the sign was sure of and the right one at a half turn.
- **A piece wound the wrong way.** `push_convex` took a piece's area about the origin in `f32`,
  each term a coordinate squared and rounded to a sixteenth of a unit at a page's coordinates —
  more than a small join's whole area — so such a piece came out wound negatively and took its
  area out of the bands it overlaps: 1 924 pieces drawn about that pixel at 64x, hairline holes
  across the stroke. The area is taken about the first corner in `f64`, where each product is exact.

The brief's hypothesis — a piece culled by a bound taken from a chord — did not hold: no piece was
culled; two were built from directions a rounding chose and one was wound by a rounding.

## 2. The tests, each red where it acts

`a_corner_stated_again_as_curves_of_no_length_keeps_its_join` (the file's six segments, scales 1,
4, 16, 64), `a_curve_that_doubles_back_is_rounded_beyond_the_turn` (the file's five segments; red
at 16 on the old sweep) and `a_small_piece_far_from_the_origin_is_wound_positively`, each red with
its fault planted back. And the paperclip's weak spot: `folding_stroke_under_a_clip.rs` holds the
page to the curve within the flattening's 20.2 levels, which the library's converter passes. The
hypothesis that the bound tightens under eight levels did not hold: the union's worst pixel there,
`(794, 566)`, reads 61 against the curve's 71.4, and that is the flattening, not the measure. What
is red on the library is the pieces held to their own flattened set, which only the crate's own
tests can reach: `the_paperclip_s_pieces_are_measured_as_their_flattened_set` draws the paperclip's
pieces sixteen times finer about the witness and holds each pixel within a level; on the library
`(796, 563)` reads 128 against 130.65.

## 3. Priced

No figure is taken: each change is a predicate or a sum over a piece's few corners, and the skip
removes work. The turn gate (`t2-turn_path`) is the record's.

## 4. The pixels that move

Nine of the 974 first pages `raster_golden` holds move, raster only, and the goldens are re-taken
with this ADR; each is a page with a folding stroke, and three of them are ADR 2268's, whose marks
the union now draws. Pixels moved and worst level, at 1x and then at 4x, against HEAD:
`asciihexdecode` 1/1, 17/49; `inks` 3/1, 3 028/27; `issue12823` 246/38, 83/27; `issue14415`
112/44, 20/5; `issue14438` 6/37, 225/118; `issue19182` 2/1, 15/14; `issue19360` 9/4, 234/68;
`issue20513` 38/12, 362/121; `pr12564` 1 713/37, none. Each page's worst pixel was drawn beside
HEAD's and read (trap 1): `issue14438.pdf`'s corner keeps its join, where HEAD drew the light line
across it; `inks.pdf`'s hairlines across its strokes at 4x are gone, as is the faint mark beside
`asciihexdecode.pdf`'s upright; `pr12564.pdf`'s nine marks, which the library drew, read the same
to the eye; and nothing reads worse.
