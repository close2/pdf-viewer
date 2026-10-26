# 2404 — A stroke painted with a ramp is drawn from the oracle's own cells, over its outline's device bounds

Status: accepted for integration. Session 1785, 2026-10-26, the oracle slot of batch one hundred and
eighteen. Clause: ISO 32000-2 §8.7.4.5.3 and §8.7.4.5.4 for what a ramp is at a point, §8.5.3.2 for a
stroke painted with the stroking colour's pattern, §8.4.3 for where a stroke's ink can lie, and §10.7,
which leaves the conversion of a colour to the device's bits to the device; no ledger row moves (the
two shading rows stay `implemented`, their notes never named the residue). Closes the residue ADR 2356
§1 left named. Context: ADRs 0165, 2148, 2310, 2325, 2340, 2341, 2356, 2357; traps 2, 13, 213, 232.
Code: `render-cpu/src/stroke.rs` (`mark_brush`, `ink_extent`), `render-cpu/src/shading.rs` (`Extent`,
`OnSurface::bounds_of_ink`, `ramp_shader`), `render-cpu/src/fill.rs` (`draw_fill`). Tests:
`render-cpu/tests/a_level_where_the_paint_is_the_pixel.rs`'s
`an_opaque_ramp_on_a_stroke_is_the_level_nearest_its_colour` and
`a_stroked_ramps_cell_is_its_colour_at_a_mitres_tip`.

## 1. The construction

ADR 2356 drew an opaque axial or non-cone radial ramp that a *fill* paints from cells — each the level
nearest the ramp's colour at the pixel centre's `t` — and left a stroke handing the same ramp to
`tiny-skia` as a gradient, because the cells need an extent and a stroke's ink is not its path. A
stroke now takes the same cells: `draw_stroke` asks `mark_brush` for its brush, which for a shading
builds `ramp_shader`'s pattern over the stroke's `Extent::Stroked`, and the stroke's every converter —
the sub-pixel bands, the long mitres, the stroked outline's fill, the library's own `stroke_path` —
reads each pixel's own cell, as the fill's converters do. The marks §8.5.3.2 makes of a subpath with
no length are filled, and take the cells over their own path (`Extent::Filled`), the coverage they
gave up multiplied in after, as an edge pixel's is.

**The extent is the stroked outline's device bounds, and it is `pdf_render::stroked_bounds`'s**: the
bound §8.4.3 states term by term — half a width along a segment and at a round or bevel join, `√2`
halves at a projecting square cap, each mitre's own tip where the limit admits it (ADR 0165), and a
dash's caps along the whole path — the same one copy `pdf_model` and `pdf_render::repeat` ask. No one
outline the library builds is every converter's: the stroker's is bevelled where `draw_long_mitres`
adds wedges, a folding curve is drawn from pieces, a dash's caps are the dashed path's. A mark the
raster cannot measure is drawn one device pixel wide — §10.7.4's band, or the library's hairline under
`scan::stroke` — so the extent reaches past those bounds by the same multiple of half *that* width the
clause allows at the stated one, the mitre limit's halves at most, with the half pixel inside which a
centre lies. A cell is a function of its own pixel alone, so any extent holding the ink draws the same
pixels; only the count of cells evaluated depends on how tight it is.

`a_stroked_ramps_cell_is_its_colour_at_a_mitres_tip` is the extent's calibration (trap 13): with the
cells bounded by the path's box grown by half the width, its two pixels inside a mitre's tip read the
nearest cell's colour, 150 where 161 is the level at their own `t`, and it fails; with this extent it
passes. `an_opaque_ramp_on_a_stroke_is_the_level_nearest_its_colour` fails against HEAD's tree, where
the gradient's store breaks a half level to even.

Left as they were, each by ADR 2356's reason: a cone, a ramp below full alpha, an axis of no length,
a geometry the library refuses as a gradient, and a shading with Table 77's `/Background`, which
`shading::shader` refuses for a stroke as it did.

## 2. The population, and what it costs

Counted by what the backend draws (trap 232), a census line printed from `mark_brush` in an export of
this tree over the cpu arm at 1x: 89 strokes on 8 page ones reach a ramp, 88 drawn from cells — 86
axial, 2 radial — and one axial stroke on `bug1019475_1.pdf` left to the gradient by the alpha rule.
Rendered before and after at 1x, every moved pixel is one level: `bug1019475_1.pdf` 1 481 pixels,
`issue19360.pdf` 1 364, `issue20513.pdf` 804, `issue13325_reduced.pdf` 147, `issue6769_no_matrix.pdf`
60, `issue6769.pdf` 44, `bug1721218_reduced.pdf` 8, `bug1703683_page2_reduced.pdf` 3; read, they look
as they did (`issue19360.pdf`'s ramp-stroked lettering, `issue6769.pdf`'s radial band).

It costs what the fill's cells cost, a cell per pixel of the extent where the gradient stage ran eight
pixels a step, over a ring past the outline that the reach adds. Counted by callgrind at 1x, one
thread, the page drawn once, whole run, HEAD then this: `issue20513.pdf` (25 strokes) 245.1 M to
245.9 M instructions, `issue19360.pdf` (23) 130.3 M to 133.0 M, `bug1721218_reduced.pdf` (16) 3 037.0 M
to 3 037.3 M. The cells are the extent's pixels on the surface, four bytes a pixel, held for one mark.

## 3. The pixels

Against HEAD's six arms, page by page, every arm exit 0 and no device digest moved: 0 rises and 27
falls of a page-lane's mean — 5 on each 1x arm, 4 on each 4x arm, every one of them a census page. The
device lane's sum of page means: `cpu` 49.5936 to 49.5886 at 1x and 15.9886 to 15.9869 at 4x; `gpu`
50.4659 to 50.4609 and 18.0714 to 18.0697; `compute` 8.7762 to 8.7712 and 4.9720 to 4.9703. The
largest: `issue6769.pdf` 0.0023 to 0.0006 and `issue6769_no_matrix.pdf` 0.0031 to 0.0009 on each 1x
arm; `issue19360.pdf` 0.0068 to 0.0063; `bug1019475_1.pdf` 0.2139 to 0.2134 on `cpu` and `gpu` 1x.

`raster_golden` re-taken: 8 pages, the raster's digest alone, every display list and report unchanged
— `bug1019475_1.pdf`, `bug1703683_page2_reduced.pdf`, `bug1721218_reduced.pdf`,
`issue13325_reduced.pdf`, `issue19360.pdf`, `issue20513.pdf`, `issue6769.pdf`,
`issue6769_no_matrix.pdf` — the census's eight and no other.
