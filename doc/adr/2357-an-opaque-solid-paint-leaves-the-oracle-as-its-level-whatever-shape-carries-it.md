# 2357 — An opaque solid paint leaves the oracle as its level, whatever shape carries it

Status: accepted for integration. Session 1761, 2026-10-26, the oracle slot of batch one hundred and
sixteen. Clause: ISO 32000-2 §10.7, which leaves the conversion of a colour to the device's bits to
the device; no ledger row moves, because the rounding is this tree's documented choice (ADR 2311).
Context: ADRs 2311, 2324, 2325, 2340, 2350, 2356; trap 257, 265. Code: `render-cpu/src/convert.rs`
(`solid`, `off_the_half`, `both_conversions`), `render-cpu/src/lib.rs` (`paint`). Tests:
`convert.rs`'s `an_opaque_grey_leaves_as_a_colour_written_at_its_level`,
`render-cpu/tests/a_level_where_the_paint_is_the_pixel.rs`'s
`an_opaque_fill_is_the_level_nearest_its_colour_whatever_its_shape`.

## 1. The premise, and what it is instead

Round 1758 (ADR 2350 §3) found `knockout_groups_test.pdf`'s group 395 — isolated, not knockout —
drawing its opaque `(0.91, 0.85, 0.7)` background's blue at 179 where the device draws 178, and read
it as the oracle drawing a fill inside a group one level above the same fill at top level
(`close-path-bug.pdf`'s `0.7 g`, 178). **The group is not the cause.** Hand-built pages of one `0.91 0.85 0.70 rg 0 0 100 100 re f` drew
179 at top level and inside an isolated and a non-isolated group, each with and without `/CS
/DeviceRGB`; only an isolated knockout group drew 178.
And `0.7 g` drew 179 as a rectangle and 178 as `close-path-bug.pdf`'s curve, as a triangle and as a
rotated rectangle. The split is the **shape**: `tiny-skia` writes a run of whole pixels under an
opaque solid paint with no mask as a `memset` of `to_color_u8`, `(c · 255 + ½)` in `f32` truncated
(`pipeline/blitter.rs`, `memset2d_color`); every other pixel is its high-precision store, `round_int`
of the `f32` product — `cvtps2dq`, nearest-even, on x86-64. `scan::fill_rectangles` hands a
rectangle to the library's rectangle converter, whose interior is that `memset`; a path's interior
reaches the pixel through the oracle's own coverage mask and so through the store. The knockout
group drew 178 because §11.4.6's stages draw each element onto a scratch through `knockout_average`.

`0.7` is 178.499 997 levels and its product is 178.5: the `memset` writes 179, the store 178. Over
the 2 295 `f32`s within four ulps of each half level, 193 are written off the level nearest them by
one conversion or the other — the unit test's calibration.

## 2. The construction

ADR 2324 §3 is the rule on the device: where an opaque source covers a pixel wholly the pixel is the
level nearest the paint; elsewhere the blend rounds the unrounded paint once. ADR 2340 §1 measured
the obvious oracle construction — round the paint to its level first — and it moved 4 804 bytes of
`bug852992_reduced.pdf` away from the device, every one an edge rounded twice. So the paint is not
rounded; it is **moved off the half**. `convert::solid` hands `tiny-skia` an opaque solid paint under
source-over whose component either conversion would write off its level as the nearest `f32`
towards that level on which both write it — one ulp in every case the test meets — and every other
component as it was. A whole pixel is then the level by either route; a partly covered pixel is
blended from a paint one ulp from the producer's, which moves it only where its own `f32` composite
lies within that ulp of a half (trap 257's shape, at an ulp rather than half a level). A paint below
full alpha, and a mode other than source-over, are handed over untouched: there the pixel is not the
paint. The device-independent half of the claim is the store's: on a target whose `round_int` rounds
half away, a product off the half rounds the same way, so the oracle's bytes no longer depend on it.

## 3. The pixels

Isolated on the `cpu` arm's 1x falls by rendering each page with ADR 2356 alone and with both: this
construction moves five pages and no other construction of this round moves them —
`knockout_groups_test.pdf` (93 891 pixels, the background of each of its first page's four groups now 178; mean
0.1825 to 0.1233), `tiling_patterns_variations.pdf` (0.0125 to 0.0055), `smask_alpha_bc.pdf` (0.0255
to 0.0000), `smask_alpha_oob_transfer.pdf` (0.0067 to 0.0000) and `ThuluthFeatures.pdf` (0.2332 to
0.2254). ADR 2356 §4 carries the six arms, which hold both constructions together.
