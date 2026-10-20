# 2100 — An unsmoothed image's pixel takes the sample its centre maps to on the page

Session 1632, slot 2 of batch ninety-nine, a pixels round on ADR 2084 §5's find, under the project
owner's switch to code improvements and bug fixes of 2026-10-14. Status: **accepted** and **built**
in `render-cpu`. Clause: ISO 32000-2 §10.7.4 (the row stays `departed`, ADRs 1060, 1360, 2058; its
note gains a sentence) and §8.9.4. Context: ADRs 0025, 0139, 0219, 1758, 2084; traps 1, 13, 134,
213, 219. Code: `render_cpu::image::centres` (new), `CpuRasterizer::draw_image`. Tests:
`render-cpu/tests/image_centre_across_a_cut.rs` (two, new).

## 1. The premise, and the hypothesis that did not hold

**Premise (ADR 2084's record, "Found, not fixed"): held.** `bug1799927.pdf` at 4× (2380 × 3368)
drawn in 2, 4 and 16 strips differs from the page in one strip at **212 pixels, every one by 255
levels**, all of them in device row 2361, none within two rows of a cut.

**Hypothesis — a transparency group or soft mask whose extent a strip computes differently, closed
by a planner rule — did not hold.** Isolated on the full-size target (trap 213), the 212 pixels
belong to one line of the page's 2 156 inline stencils — 46 of them cross the row — 7 × 10 samples
each, placed by `[-4.361 0 0 -6.23 e 256.609]`: at 4× a sample row is 2.492 device rows, and the
edge between sample rows 1 and 2 lies at device row 2361.500 — on the centre of row 2361, to the
decimals the file states. The
mark is drawn by `draw_image` as a pattern, and `tiny-skia` maps a pixel's centre back from the
first row of the surface it draws into, in `f32`; the strip below the cut at 1762 states that
centre as row 599.5 and the page as 2361.5, the two sums round in different binades (ADR 0219's
mechanism, on a paint rather than a path), and on a centre that close to a sample's edge the strip
took sample row 1 — transparent — where the page took row 2, opaque. No group, no mask, no
clip and no planner row is involved: images are not marks the planner asks about, and the cut
crosses none of them.

## 2. What the clause owes, and the choice

§10.7.4: "The position of the centre of such a pixel -in other words, the point whose coordinate
values have fractional parts of one-half -shall be mapped back into source space to determine how
to colour the pixel." The pixel is the page's — "[p]ixel boundaries always fall on integer
coordinates in device space" — and a page's division into strips, or a clip's rows into a band, is
this program's choice, so the sample a pixel takes is owed as a function of the pixel's place on
the page and of nothing else. That is a requirement, not ADR 0219's residue: ADR 0219 kept the
whole page's arithmetic because the coverage it measured lay *between* the two drawings' values,
with no answer to prefer; here the clause names the computation, and it can be carried out without
reference to where a surface begins.

**Built**: for every image `pdf_render::Image::is_smoothed` leaves unfiltered, `image::centres`
composes the mark's matrix and the page's in `f64` from their own entries, states each pixel centre
in the page's device coordinates (a surface's first row added back as a whole number), maps it onto
§8.9.4's sample grid, and hands the library one sample per device pixel as a pattern at whole
pixels, where its sampler cannot change one. The square's region and its anti-aliased edge are the
converter's as before: the unit square is carried onto the surface by `Path::transform`, which
maps a point by the same `map_points` the converter applies to a matrix it is handed.

**Two decisions inside it**, each stated where it is made:

- **A centre on the edge two samples share.** §8.9.4 gives each sample "one square unit" with its
  upper-left corner at its own indices and states no side for a shared edge; the centre is taken
  as the sample whose corner it is, as §10.7.4 identifies a pixel by "the location of its corner
  with minimum horizontal and vertical coordinates". A choice by analogy, not a derivation.
- **The division by the matrix comes last.** An image at one sample a pixel from a half-pixel
  origin puts every centre exactly on a sample edge (`issue269_2.pdf`'s 100 × 100 grids at
  `e = 353.5`); a first formulation that inverted the matrix and multiplied back moved 3 200 of that
  page's pixels by up to 46 levels at 1×, at whichever columns `1/100 × 100` rounded under 1. With
  the products first and one correctly rounded division, an exactly placed centre is computed
  exactly, and that page is HEAD's to the byte. `a_centre_on_a_samples_edge_takes_the_sample_whose_corner_it_is`
  holds it over two hundred origins, and fails with the first formulation planted back.

**What it does not reach**: a *filtered* image (§8.9.5.3's `/Interpolate`, or a reduction) is still
`tiny-skia`'s bilinear at a shifted origin — ADR 0219's residue, a level or so, held by ADR 1758's
bound; and on a centre the file's own decimals put on a sample edge, the `f32` the interpreter
parsed them into decides the side, now the same side in every division.

## 3. Measured

On the round's export of HEAD plus this change alone (ADR 1890), HEAD's figures from
`/home/AI/arms-1631/` and from an export of HEAD built beside it:

- **The witness**, `bug1799927.pdf` at 4×, drawn whole and in 2, 4 and 16 strips: 212 pixels of
  255 levels at each division → **0 at each**. Drawn whole, 379 of its pixels moved from HEAD's,
  every one a whole sample: the 212 of row 2361, the 159 of row 2984 and eight of column 1988, each
  a centre within an `f32` rounding of a sample's edge, now on the side the parsed matrix puts it.
  Looked at, the stencils are the lettering they were, a bar a pixel shorter or longer.
- **`issue6621.pdf` at 4×** is the other page that moved: 1 461 pixels in five columns 81 pixels
  apart, its 250-sample image drawn 405 pixels wide, so every fiftieth sample's edge is a pixel's
  centre exactly (`40.5 × 250 / 405 = 25`). HEAD's `f32` took sample 24 there; the corner rule
  takes 25. Looked at, the seal is the seal.
- **The six arms**: 0 frame digests differ in any arm (968, 968, 968, 965, 965, 965 pages) —
  the device and compute lanes do not draw through `render-cpu`. The mean error against the
  oracle, which is this backend at sixteen strips, moves on no page at 1× and on four or five at
  4×: `issue6621` 0.0000 → 0.0218 in each (the five columns above, which the three lanes, each
  sampling in `f32`, still take from the lower sample), `bug1799927` 0.0035 → 0.0075,
  `issue13561_reduced` 0.0002 → 0.0000, `issue840` −0.0001 to −0.0002, `stamps` +0.0001 on the
  processor lane.
- **The division census at 4×**, every tracked first page drawn whole and in 2, 4 and 16 strips
  (`stroke_cut_census --draw`, 957 of 974 drawn): HEAD 292 pages move at some division, 11 of
  them past one level, the worst `bug1799927` at 255; this 291 and 10, the worst 12. The one
  page whose census changed is the witness. The ten, each by its most pixels and most levels at
  any division: `issue12810` 7 150 and 12, `issue9418` 10 317 and 2, `issue12295` 8 699 and 3,
  `bug1885505` 2 720 and 2, `comments` 2 623 and 2, `highlights` 2 613 and 2, `issue14297` 1 197
  and 2, `issue1350` 250 and 2, `bug1146106` 24 and 2, `bug1755507` 9 and 4.
- **`raster_golden`**: held 974, moved 0. Its census at 1×: 173 pages moving at some division, at
  most 943 pixels and 3 levels — ADR 2084's figures — and `pdfjs_wikipedia.pdf` inside both bounds
  (369 of 484 704 pixels, one level), so it leaves `PAST_THE_BOUND`, which names ten.
- **`Underway::in_strips`** over the corpus at four strips: 610 first pages finished in strips, 19
  given up for a mark across a cut — HEAD's. No planner row moved: images are not planned marks.
- **What it costs**, counted by `callgrind_rasterise`, one draw on one thread, HEAD → this:
  `bug1799927` at 4× 756.67 → 776.31 million instructions (+2.6%), `issue6621` at 4× 68.59 →
  70.14 million (+2.3%), `firefox_logo` at 4× (a filtered image) unchanged; a page that is one
  64 × 64 image at 4× (8.0 million pixels) 1 524.53 → 1 548.67 million axis-aligned (+1.6%),
  1 163.66 → 1 214.80 turned a quarter (+4.4%), and 1 409.60 → 1 760.66 sheared (+24.9%), the one
  placement asked per pixel. A sheared, magnified, unfiltered image is the price's worst case and
  the rarest of the four; the axis-aligned case, which a scan viewed at magnification is, is the
  common one.

## 4. What it leaves

- **The ten pages past one level at 4×** are ADR 1758's kind — several rounded results in one
  pixel, a photograph under a blend, a level at a shifted origin — with one exception named for a
  round that owns it: `issue12810.pdf` is 10 368 device rows tall at 4×, past the library's 8191,
  and its pixels move in the first strip as well, whose origin is the page's — so a decision is
  taken from the surface's extent (trap 146's shape), on hatching strokes a quarter of a pixel wide
  dozens of which share a pixel. Not isolated further here.
- **A filtered image** keeps ADR 0219's residue under `tiny-skia`'s bilinear filter, inside ADR
  1758's bound on every page the census draws.
- **The library's one-row rectangle** (ADR 2084 §5), measured again on the library alone over
  2 079 dyadic one-row rectangles: every one is stated under its area, 1 519 of them by more than
  half a level, at most 0.984 of a level there (and 1.5 at ADR 2084's non-dyadic `19.6–19.9`); the closed
  form `scan::mask_rectangle` writes is within half a level of each. The measurement agrees with
  routing such a rectangle to the closed form, and it is not routed here: it moves a level on every
  sub-pixel horizontal rule of every page at 1×, which is a re-held golden of its own beside the
  coverage arithmetic slot 5 of this batch owns, and it is left to a round that owns `scan.rs`.
