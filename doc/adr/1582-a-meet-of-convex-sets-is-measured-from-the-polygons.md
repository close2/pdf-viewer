# 1582 — A meet of convex sets is measured from the polygons, and the regions' thirty milliseconds have no cheaper exact construction left on the walk

Status: accepted. Session 1373. Answers ADR 1567 section 2, which recorded the residue regions'
30 ms ceiling on `bug1721218_reduced.pdf`'s 1× frame and read the rest as each dot's exact fill.
Supersedes nothing; amends ADR 1567 section 2 in what the 30 ms is made of, below.
Context: ISO 32000-2 §10.7.4, §8.5.3.3, §8.5.4, §10.7.2; ADRs 1467, 1479, 1491, 1517, 1541, 1555,
1567; `doc/habits/measuring.md` 63, 65 and 68; trap 50.
Code: `raster/crates/raster-gpu/src/raster/convex.rs` (new: `Convex`, `Convex::of`, `ConvexMeet`,
`ConvexMeet::area_in_pixel`, `Work`), `encode/meet/convex.rs` (new: `ChainLink`, `areas`,
`pieces`), `encode/meet/deferred.rs` (`ExactInputs::areas`, `ExactMeet::new`), `encode/meet.rs`
(`Encoder::meet_residue`), `encode/clips.rs` (new: `Encoder::chain_links`), `raster/fill/rows.rs`
(`RowIndex::subpaths_meeting` made crate-wide), `raster.rs`.
Tests: `raster/convex/tests.rs`'s `a_convex_meet_is_the_general_meet_byte_for_byte` (50 000
pixels, over 2 000 of them cut by both sets), `what_is_not_one_convex_polygon_is_declined`,
`a_square_reads_its_closed_form`; `encode/meet/convex/tests.rs`'s
`a_disc_under_many_dots_meets_as_the_general_meet` and `a_meet_of_other_sets_is_the_general_meets`;
the corpus gate's per-page digests on six arms.

## 1. Counted first (habit 63): what the page's regions and meets are

The brief's premise was 3 515 regions of one 37-point dot each, a dot being one shape under 3 515
transforms. The content stream and the encoder say otherwise, in three ways.

- **A dot is not one shape.** The page states 3 493 clip paths; 3 281 are one dot family (four
  cubics and four lines of no length). Each is written afresh in page coordinates at the
  producer's 0.001 pt: taken relative to its first point, they are **3 280 distinct shapes**, one
  pair alike; their first points take 3 277 distinct phases at 0.001 and all 256 at 1/16 of a
  pixel. All share one transform. A memo keyed by the sub-pixel offset (ADR 1517's keying) would
  hand one dot another's bytes, each differing by up to 0.003 pt, about half a level along a pixel's
  edge — not exact, and not taken.
- **The circle is not the file's.** The file states each dot and each disc as four cubic Béziers;
  the 37-point polygon is this tree's flattening of them (§10.7.2), and the oracle's reference
  measures that polygon's set. Neither is a circle, so a closed form for a circle's coverage is
  exact for nothing on the page. Discarded.
- **Two populations, not one.** The 3 515 regions a frame are single-dot chains (37 or 41 points,
  2 × 1 to 3 × 2 pixels) under axial shadings that cover their whole tile: no pixel is cut by both
  sets, so they are filled and met by `min`. The exact meets are another 3 256 a frame: a radial
  shading's disc (33 points) under **one clip of 3 014 dots** (another of 232), about 11 400
  pixels a frame cut by both, and `exact_areas` spent **45 000 instructions a pixel** on them (callgrind, one
  thread, 1.03 G over the zoom pair).

## 2. Where the 30 ms is, by the clock

Scratch spans on an export of HEAD (removed since), pinned, minima of 3 rounds, the chromatic
render of the 1× frame: the region fills 9.1 ms, the links' flattening 4.4, the chain's edges 4.7
(one 4.98 MB build for the 3 014-dot clip, then a charge per meet), pricing 1.7, chain numbers
1.8, meet words 1.4, the settle's wait for the helpers 1.3; the black render 2.7 + 0.9 + 1.3 in
its kept lookups. A probe arm making no exact area at all (wrong bytes) took the frame from
86.7–90.0 to 81.0–87.7 ms: **the exact meets are about 5 ms of the 30**, because ADR 1541 already
makes them beside the walk.

## 3. Kept: a meet of convex sets is measured from the polygons, on the helpers

A set that is one closed polyline turning one way and going round once is the intersection of the
half-planes its edges bound, and so is the intersection of several such sets: a convex polygon,
which `ConvexMeet` cuts to each pixel (Sutherland–Hodgman, the pixel's own sides placed exactly)
and measures by the shoelace from the pixel's corner. §8.5.3.3's two rules agree on it. A link of
many subpaths is taken as the subpaths whose boxes meet the tile (`RowIndex::subpaths_meeting`),
each convex and their boxes apart, so that inside the tile it is their disjoint union; the meet is
then the sum over one polygon per choice of a piece of each link, at most 16.

- **Convexity is decided exactly or not at all.** `Convex::of` reads each turn's sign under
  Shewchuk's `orient2d` bound, `(3 + 16ε)ε` of the products' magnitudes, and declines a turn it
  cannot certify, a reflex corner, a turn back along an edge, a polygon going round twice, or more
  than one subpath. On this page some flattened dots and discs have a reflex corner of a few
  10⁻⁵ px² where two of the producer's cubics join; those meets are declined and met as before —
  68 of the zoom pair's 6 512, the other 6 444 measured from the polygons.
- **It answers exactly the meets the general meet answers.** It runs inside
  `ExactInputs::areas`, on the frame's helpers, after the walk has built everything the general
  meet reads — so a declined meet is the general meet unchanged and the walk's work is HEAD's. The
  mark is measured only where `2 · rows + 3 · points` is within its edge limit, which bounds the
  charge `RowEdges::of` would make (a line crosses a convex boundary twice), so a mark the general
  meet leaves at `min` is left there here too.
- **Exact to the same precision, not byte-identical by construction.** Both constructions are the
  set's area in `f64` from the same `f32` points, far below a level; a byte could differ only where
  that area lies within a few units in the last place of a rounding boundary. The tests above find
  none in 50 000 pixels; the corpus finds none on six arms (section 5).

**Instructions**, callgrind, one encode thread, the zoom pair: 5.649 G → 5.224 G, of which the
settle's exact areas 1.044 → 0.611 G. What is left of them is each disc cut by each dot piece its
tile meets, 2.65 pieces a meet, at the polygons' 33 × 37 corners — not the pixels.

## 4. Tried and not kept

- **Deciding convexity on the walk**, so that a convex meet's chain needs no edges: the decision
  cost 7.7 ms a frame on the walk's thread for the 4.7 ms of edges it could remove, and the
  declined meets still need the whole chain's edges. Moved to the helpers instead (section 3).
- **A convex region's coverage measured as a polygon** in place of the fill: 1.20 µs a dot against
  the fill's 1.50 on 4 000 of the page's dots (release, one thread) — a fifth of 9 ms, not
  byte-identical by construction, and a second fill for one shape class. Not taken. **This is the
  region fills' floor**: the fill is one pass of exact trapezoid deposits over 37 tiny edges and a
  set question already as small as its inputs (habit 63), every region is filled once a frame and
  handed to the black render (ADR 1529), and no exact construction measured cheaper than itself.
  What is left on the walk is the flattening (bytes fixed by ADR 0008), the chain's edges for the
  meets that decline, and the kept lookups — each under 5 ms.

## 5. Measured

Exports of HEAD and of HEAD with this change in their own target directories (`md5sum`-distinct),
`zoom_frame`, pinned, minima of 5 rounds, four interleaved runs, load 2.8 to 3.4:

| `bug1721218_reduced.pdf` | HEAD | change | the CPU backend |
|---|---:|---:|---:|
| the 1× frame, GPU lane | 81.3–83.3 ms | 79.5–80.8 ms | 47.3–47.7 ms |
| the 1.25× step | 78.4–79.9 ms | 77.2–77.8 ms | 56.2–56.7 ms |

**The 1× frame is 1.68× the CPU backend, from 1.72×; the step 1.37×, from 1.40×.** A second
sitting read the same direction by 1 to 1.5 ms.

**Corpus.** `render-raster --test corpus` on the CPU, GPU and compute lanes at 1× and 4×, per-page
digests compared by name against exported HEAD: **0 pages moved on any of the six arms**, verdicts
equal, one-versus-many 0.
