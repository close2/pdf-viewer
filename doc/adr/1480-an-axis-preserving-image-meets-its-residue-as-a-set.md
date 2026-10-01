# 1480 — An axis-preserving image meets its residue as a set, and a horizontal edge bounds a band

Status: accepted. Session 1322. Takes ADR 1467's one exception — the image lane meeting a residue by
`min` — and amends ADR 1444 there. Corrects ADR 1467's band construction for horizontal edges.
Supersedes nothing.
Context: ISO 32000-2 §8.3.3, §10.7.4, §11.6.4.2; ADRs 0011, 1435, 1444, 1456, 1467, 1479;
`doc/questions/A76`.
Code: `raster/crates/raster-gpu/src/encode/rare.rs` (`Encoder::image_residue`), `encode/meet.rs`
(`Encoder::meet_residue_with_rectangle`, `rectangle_cut`), `raster/meet.rs` (`levels_between`,
`Level`, the cut in `area_in_pixel`).
Tests: `raster-gpu/tests/an_image_meets_its_residue_as_a_set.rs` (new), `raster/meet.rs`'s
`a_horizontal_edge_through_the_pixel_bounds_a_band` (new); `tests/common/meet.rs` (new) holds the
closed-form arithmetic both residue fixtures share, moved out of `residue_meets_a_mark_as_a_set.rs`.

**`crates/render-cpu/` was not opened.** Every expected value is a polygon's area in a pixel.

## 1. Which images are polygons in the lane that draws them

§10.7.4: "Subsequent painting operations shall affect a region that is the intersection of the set
of pixels defined by the clipping region with the set of pixels for the region to be painted", and
§11.6.4.2 says what an image's region is: "[f]or images … the shape shall be 1.0 inside the image
rectangle and 0.0 outside it". `image.wgsl` draws two cases (ADR 0011):

- **An axis-preserving placement** — a scale, a flip, a quarter turn — maps the unit square onto an
  axis-aligned rectangle, and the shader takes each pixel's exact overlap with that rectangle met with
  the clip rectangle. That set is a polygon, so ADR 1467's meet applies to it: built.
- **An oblique placement** is painted in the pixels whose centre maps inside the unit square: a set
  that holds every pixel whole or misses it, which `min` meets exactly. The parallelogram's own edges
  are not drawn analytically on this lane at all, clipped or not; that is ADR 0011's stated cost of the
  rare case. Meeting a residue with the parallelogram's area while the same edge elsewhere stays hard
  would draw one edge two ways, so it is not done here. Antialiasing oblique images is the lever, and
  it would bring the meet with it.

## 2. The construction

The image's residue tile is computed as before. Where the tile's byte is fractional and so is the
rectangle's overlap (computed as the shader computes it), the byte is replaced by the area of the
rectangle's intersection with the chain in that pixel — `area_in_pixel` over the rectangle's edges and
the chain's kept edges (ADR 1467), no larger than the residue's byte. The shader's `min` then reads
that area, which is never above the rectangle's own overlap. Every other pixel of the tile keeps the
residue's byte, so an image the residue holds whole or misses whole draws exactly what it drew.

## 3. A horizontal edge bounds a band

Building the fixture found a defect in ADR 1467's band construction. The bands of a pixel were cut
where an edge *through* the pixel starts, ends or crosses a side; horizontal edges were dropped as
crossing no ray. But a horizontal edge running across the pixel between two sides outside it changes
a set's inside at its height while no edge through the pixel starts there: the rectangle
`[-3, 9] × [2.2, 9]` against `x ≤ 2.6` in pixel `(2, 2)` read 0.6 instead of 0.48. Each row's
horizontal edges strictly inside it are now kept beside its edges, and one crossing the pixel cuts
its band. The path lane's meet is the same function, so this can move a path's pixels too (section 4).

## 4. Fixture and corpus

`a_quarter_turned_image_under_a_polygon_clip_meets_each_pixel_as_a_set`: a one-texel opaque image
turned a quarter turn onto `[2.35, 12.65] × [4.2, 13.9]` under the 64-gon — every pixel within half a
level of the closed form, on the fan-out at one thread and four and on the walk's tile, the same
bytes. With the meet switched off it failed: pixel `(4, 4)` drew 204 against 203.22.

Corpus (`render-raster --test corpus`, per-page digests against ADR 1479's build, exported trees): at
1× four pages move on all three coverage lanes — `ContentStreamNoCycleType3insideType3.pdf`,
`bug1771477.pdf`, `issue17730.pdf`, `pattern_text_embedded_font.pdf` — and at 4× three of them. At 1×
every one is section 3's horizontal-edge band: rebuilt with the image meet switched off, the four draw
the same bytes, so **at 1× no corpus page has an axis-preserving image whose rim and a residue's cut
one pixel**; the 4× pages are three of the same four. The fixture is the image meet's witness. Against the oracle, which meets two bytes: two
pages closer (`bug1771477.pdf` 0.0223 → 0.0220 at 1×, `pattern_text_embedded_font.pdf` 0.0099 →
0.0098), the rest equal to four places; verdicts and one-versus-many unchanged.
