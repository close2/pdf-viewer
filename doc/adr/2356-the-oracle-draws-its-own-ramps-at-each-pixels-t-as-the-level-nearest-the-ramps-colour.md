# 2356 — The oracle draws its own ramps, at each pixel's `t`, as the level nearest the ramp's colour

Status: accepted for integration. Session 1761, 2026-10-26, the oracle slot of batch one hundred and
sixteen. Clause: ISO 32000-2 §8.7.4.5.3 and §8.7.4.5.4 for what a ramp is at a point, and §10.7,
which leaves the conversion of a colour to the device's bits to the device; no ledger row moves (the
two shading rows stay `implemented`, their code amended). Takes ADR 2341's first way out, which it
supersedes in part: §2's second lever, a patched `tiny-skia`, stays unpriced. Context: ADRs 2148,
2310, 2311, 2324, 2325, 2340, 2341, 2350, 2357, 2367; traps 2, 257, 265. Code:
`render-cpu/src/shading.rs` (`ramp_shader`, `Sweep`, `RampCells`, `level_of_colour`),
`render-cpu/src/fill.rs` (`draw_fill`), `render-cpu/src/lib.rs` (`brush`). Tests: `shading.rs`'s
`a_ramps_cells_are_the_walks_colours`; `render-cpu/tests/a_level_where_the_paint_is_the_pixel.rs`'s
`an_opaque_ramp_is_the_level_nearest_its_colour` and `a_ramps_cell_is_its_colour_at_the_pixels_own_t`.

## 1. The construction

An axial or radial ramp that a fill paints is no longer handed to `tiny-skia` as a gradient. For each
pixel of the shape's device bounds on the surface, on the page's own grid (ADR 2148), the pixel's
centre is taken back into the shading's space and its `t` is §8.7.4.5.3's projection onto the axis —
with the clause's "otherwise, t is undefined and the point shall be left unpainted" for a point past
an end `/Extend` refuses — or §8.7.4.5.4's greatest admissible blend circle through
`pdf_render::blend_parameter`. The cell holds the level nearest `Ramp::colour_at(t)`, the
`nearest_level` ADR 2340 gave the function grid, which is the byte the device's shading lane writes
for an opaque paint (ADRs 2310, 2325). The cells are a pattern positioned in the path's space at one
per device pixel, sampled nearest, so the fill's every converter — the rectangle closed form, the
sub-pixel bands, the collapsed marks, the path converter — draws the shape as it did and reads the
pixel's own cell. At a partly covered pixel the blend takes the level times the coverage, which is
the device's own order: its shading lane multiplies the opaque level by the shape (`fs_main`).

Left as they were, each by a reason: a **cone** is `fill_radial`'s raster, which both backends draw
from the same bytes; a **ramp below full alpha** — a constant alpha folded into its stops — keeps the
gradient, because the library multiplies the colour by its alpha in `f32` and the blend rounds once,
as the device hands a translucent paint over (ADR 2350), where a cell would round first; an axis of
no length and a radial geometry the library refuses as a gradient keep the gradient's own answer and
refusal; a shading with Table 77's `/Background` is `fill_with_background`'s; and a **stroke** with a
ramp still strokes a gradient, because its cells would want the outline's bounds rather than the
path's, which this round did not build — a residue of ADR 2341's store, left named. The first run of the arms drew the translucent ramps from cells too and rose
three pages (`issue18956.pdf` 0.0007 to 0.0250 on the `cpu` arm at 1x, `issue7821.pdf`,
`alphatrans.pdf`), which is why that case is excluded rather than included.

## 2. What it costs

A cell per pixel is evaluated in the oracle where the library's gradient stage ran eight pixels a
step. Walking a ramp's stops per pixel took `issue14165.pdf` (232 stops, one full-page axial) at 4x
from about 0.33 s to 0.89 s single-threaded, best of six. `RampCells` finds the stop by halving after
trying the interval the last `t` fell in, and takes the last cell for a `t` equal to the last one — every
pixel of a row across an axis that does not move along it — and is held bit for bit to the walk by
`a_ramps_cells_are_the_walks_colours`. Counted by callgrind at 1x, one thread, whole run, HEAD then
this: `issue14165.pdf` 610.2 M to 269.8 M instructions, `issue840.pdf` 713.7 M to 729.7 M,
`bug852992_reduced.pdf` 99.4 M to 136.2 M, `radial_gradients.pdf` 264.0 M to 304.3 M. The cells are
the shape's bounds on the surface, four bytes a pixel, held for one fill.

**Ties a stored alpha makes are not this construction's to close.** `alphatrans.pdf` draws a ramp over
a group whose alpha 0.5 both backends store as byte 128, and the composite alpha over it is exactly
191.5, which the oracle's store rounds to even (192) and RADV to 191, against an exact 191.25 (ADR
2367): a ramp drawn per pixel through a byte-stored alpha keeps those ties, and the page, a translucent
ramp, is outside this construction in any case.

## 3. What moved besides the five

ADR 2357's solid paint moved five pages and no other; every other pixel this round moved is a ramp's.
Rendered before and after at 1x, the ramp pages move one level at most — `issue840.pdf` 3 046 pixels,
`bug852992_reduced.pdf` 638, `ShowText-ShadingPattern.pdf` 999, `issue14165.pdf` 98, nothing more than
a level — and look as they did (`bug852992_reduced.pdf` read).

## 4. The pixels

Against HEAD's six arms, page by page, both ADRs together, every arm exit 0 and no device digest moved:
0 rises and 104 falls of a page-lane's mean — 21 on `cpu` 1x, 20 on `gpu` 1x, 20 on `compute` 1x, 15,
15 and 13 at 4x. The device lane's sum of page means: `cpu` 49.7155 to 49.5936 at 1x and 16.1005 to
15.9886 at 4x; `gpu` 50.5858 to 50.4659 and 18.1829 to 18.0714; `compute` 8.8990 to 8.7762 and 5.0839
to 4.9720. ADR 2341's five rises are falls: `bug852992_reduced.pdf` 0.0196 to 0.0176 on each 1x arm,
below the 0.0191 it read before ADR 2325; `issue840.pdf` 0.1144 to 0.1134 and 0.1150 to 0.1140 on the
`cpu` and `gpu` 1x arms (and 0.0229 to 0.0219 on `compute`). `knockout_groups_test.pdf`'s background
is 178 (ADR 2357): 0.1825 to 0.1233 on `cpu` 1x, 0.1374 to 0.0769 at 4x. `function_based_shading.pdf`
did not move.

`raster_golden` re-taken: 35 pages, the raster's digest alone, every display list and report
unchanged — `S2.pdf`, `ShowText-ShadingPattern.pdf`, `ThuluthFeatures.pdf`, `bug1019475_1.pdf`,
`bug1703683_page2_reduced.pdf`, `bug1721218_reduced.pdf`, `bug852992_reduced.pdf`, `close-path-bug.pdf`,
`gradientfill.pdf`, `issue13520.pdf`, `issue14165.pdf`, `issue14297.pdf`, `issue17065.pdf`,
`issue17069.pdf`, `issue18032.pdf`, `issue18529.pdf`, `issue18816.pdf`, `issue19360.pdf`,
`issue5804.pdf`, `issue6961.pdf`, `issue7339_reduced.pdf`, `issue7847_radial.pdf`, `issue8111.pdf`,
`issue840.pdf`, `issue9243.pdf`, `knockout_groups_test.pdf`, `pattern_text_embedded_font.pdf`,
`radial_gradients.pdf`, `shading_extend.pdf`, `smask_alpha_bc.pdf`, `smask_alpha_oob_transfer.pdf`,
`stamps.pdf`, `tiling_patterns_variations.pdf`, `transparency_group.pdf`, `type4psfunc.pdf`.
