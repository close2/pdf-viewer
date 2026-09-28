# 1374 — The floor is owed per shape, and a square's corner pixel is the rectangle converter's level

Status: accepted. Session 1268. Narrows ADR 0419's floor where `scan` measures rectangles; ADR
0476's choice of the library's rectangle converter for a fill stands, with its alternatives priced.
Context: ISO 32000-2 §10.7.4, §11.6.2, §8.4.3.2; ADRs 0419, 0476, 0590, 1064, 1082, 1348; trap 56;
habit 50.
Code: `crates/render-cpu/src/scan.rs` (`owed_a_floor`, `largest_overlap`, `stated`,
`mask_rectangle`, `mask_shared_rectangles`, `mask_summed_pixels`).
Tests: `crates/render-cpu/tests/abutting_rectangles.rs`; `scan.rs`'s
`a_rectangle_is_lifted_only_where_the_whole_of_it_would_disappear`; `lib.rs`'s
`a_small_disc_s_pieces_add_up_to_its_set_within_the_flatness`.

## 1. What round 1263 saw, and what it was

A redaction cuts a filled square into eight rectangles around the region: one path, interiors
disjoint, footprints sharing pixels, so `scan::Exact::Shared`, measured in closed form (ADR 0590).
The square before the cut is one rectangle, drawn by `tiny-skia`'s rectangle converter (ADR 0476).
Swept over 600 random squares and regions at 150 dpi, the two differ in two ways, and neither is
trap 56's (no sample is lost where pieces abut):

- **A single pixel at an outer corner**, one level in the pieces and none in the square: the pixel
  holding the corner, covered by `0.0833 × 0.0417` of it, 0.885 of a level. Rounded to nearest the
  clause's area is one level, so the **pieces are right**. The square reads nothing because the
  library's 8.8 fixed point truncates a corner's product — the one level ADR 0476 measured and
  stated between the two arithmetics.
- **A whole column or row of one level outside an edge**, in the pieces only. `45.6 · 150/72` is
  `95.0` in the reals and `94.99999` in `f32`, so the edge reaches a ten-thousandth into column 94,
  and `mask_rectangle` lifted every positive coverage under a level to one level (ADR 0419). That
  was the defect: the square has not disappeared, and its sliver of residue was painted as ink.

## 2. The decision: the floor is asked of the shape

§10.7.4's purpose is stated of a shape — "This ensures that no shape ever disappears as a result of
unfavourable placement relative to the device pixel grid". So `owed_a_floor` asks it once per mark:
a mark none of whose rectangles has a pixel reaching one level is lifted, as ADR 0419 decided; every
other mark's pixels are rounded to nearest, which is the closed form's level and what
`crate::area` already writes (ADR 1082). ADR 1064 met the same residue in a clip and routed that
region to the converter; this is the rectangle's own remedy. Planted back (the floor on every
pixel), `eight_abutting_pieces_read_the_closed_form_at_every_pixel` fails at `(94, 147)`, 1 level
where the area is 0.0001.

## 3. Why the corner stays one level

Making the single rectangle exact was built and measured, `callgrind_rasterise`, five draws: through
the coverage buffer as `Exact::Shared` goes, `colors.pdf` +183%; as nine bands each filled at its
own coverage by the library, ISO 32000-2 page 6 +48% (a pipeline per band). Both would move nearly
every first page with a fractional rectangle edge by a level. ADR 0476's choice stands, and
`the_square_drawn_whole_is_within_one_level_of_the_closed_form` holds it to its stated bound.

## 4. What moved, and what it cost

`raster_golden`: 11 first pages, `raster only`, every moved pixel one level lighter and next to a
rectangle's or image's edge (`colorkeymask.pdf`'s row above its image; `issue8187.pdf`'s column
beside a bar), each diffed against HEAD rendered from its own export. Rows regenerated. Cost against
HEAD: page 101 +0.025%, page 6 +0.011%, `colors.pdf` −0.002%, `issue12295.pdf` +0.03%.

## 5. A small disc's pieces (ADR 1348)

A radius-4 and a radius-1 circle at `8 w` are disks of radius 8 and 5. The union of the pieces
`folding_stroke_outline` states, measured exactly by `crate::area` at the scale they were flattened
for, is short of the Bézier circle's set by 0.150 and 0.056 units at scales 1 and 2 (radius 4) and
0.050 and 0.014 (radius 1), against the bound `FOLD_FLATNESS` promises — the rim in device pixels
times a sixty-fourth — of 0.785, 0.393, 0.491 and 0.245. The derivation holds; nothing changed.
