# 2340 — A cell of the oracle's function grid is the level nearest its value, and a coverage keeps its product

Status: accepted for integration. Session 1753, 2026-10-26, the oracle slot of batch one hundred and
fifteen. Clause: ISO 32000-2 §10.7, which leaves the conversion of a colour to the device's bits to
the device, and §8.7.4.5.2 for the function-based shading the grid holds; no ledger row moves, and
§10.7.4's note is made exact about one residue. Context: ADRs 0339, 0408, 2310, 2311, 2324 §4, 2325,
2331, 2341; trap 13. Code: `render-cpu/src/shading.rs` (`nearest_level`, `sampled_shader`). Tests:
`render-cpu/src/shading.rs`'s `a_component_is_the_level_nearest_its_exact_value`,
`render-cpu/tests/sampled_shading.rs`'s `a_cell_holds_the_level_nearest_the_producers_own_value`.

## 1. Where the oracle's level is taken, which is not `scan::level_of`

The brief named `scan::level_of` as the rounding behind ADR 2324 §4's eleven rises. It is not on
their path: it turns a **coverage** into a mask byte, and its only change of answer under an exact
product, tried first, moved two bytes of `issue840.pdf` at 4x and none of the three pages' rises.
The oracle turns a **colour** into a byte in three places, none of them its own arithmetic until now:
a solid paint and a ramp, each stored by `tiny-skia`'s high-precision pipeline as the `f32` product
with 255 rounded to nearest-even (ADR 2341); and a cell of a function-based shading's grid, which
`sampled_shader` wrote through `to_color_u8`, `(c · 255 + ½)` in `f32`, truncated. Where the `f32`
product of a component a hair below a half level lands on the half, the first breaks it upwards half
the time and the second always. A solid paint's colour on the pages read had
no such tie — `issue840.pdf`'s eight opaque colours each lie at least a third of a level from a half
level — and rounding it beforehand rounded every antialiased edge twice (trap 257 on the oracle's side): tried on
the three pages, it moved 4 808 bytes of `bug852992_reduced.pdf` at 1x, 4 804 of them away from the
device. So the solid paint stays the library's.

## 2. The construction

`sampled_shader` writes each cell as the level nearest its premultiplied component's exact value,
half away from zero — ADR 2311's rule, in ADR 2325 §2's form: `⌊510 c⌋` taken in `f64`, where a
24-bit significand times 510 is exact, and halved rounding up. A cell is drawn at one per device
pixel (ADR 0408), so the byte a cell holds is the pixel inside the shape, and it is now the byte the
device's function lane writes for the same answer. The unit test holds the function to the exact
value over the 765 greys ADR 2325 proved the device with and counts the greys `to_color_u8` broke;
the shading test draws four such greys and `0.7` through a flat producer and reads each square's
interior — red on HEAD (129 for the grey 128.499 999 94 levels), green after.

## 3. Why a coverage keeps its `f32` product

Raster's coverage accumulators quantise the way the oracle's mask does, `(cov · 255).round()` in
`raster/fill.rs` and `raster/fill/exact.rs`. Taken exactly on the oracle's side alone, `scan::level_of` and `area`'s copy moved one pixel of
`issue5501.pdf`'s text at 1x from the level all three lanes draw (64) to the one above it, a rise of
0.0001 on the `cpu` and `gpu` arms. An exact coverage is therefore a change to both sides in one
commit, owned by a round that owns both, and not this one; and it would not lift §10.7.4's residue
in any case. ADR 2331 named "`scan::level_of`'s rounding" among the three residues of the area
sentence, and that residue is the quantisation itself: rounded to the nearest of 255 levels, a
partly covered pixel deposits up to half a level less than its area, however exactly the nearest is
found. The note says so now; the row stays `departed`.

## 4. The pixels

Against HEAD's six arms, page by page, every arm exit 0: no device digest moved on any arm, and one
page-lane in each arm moved its mean — `function_based_shading.pdf`, 0.0314 to 0.0263 on the three 1x
arms and 0.0087 to 0.0075 on the three 4x arms, below the 0.0285 and 0.0080 it read before ADR 2325
gave the device the exact level. No other page-lane rose or fell to four decimals; the `cpu` arm's
sum of page means is 49.8050 at HEAD and 49.7999 here at 1x, 16.1914 and 16.1902 at 4x. Of ADR 2324
§4's eleven rises the six on this page are gone; the five on `bug852992_reduced.pdf` and `issue840.pdf`
are the ramp store's, untouched, and ADR 2341 defers them. The page itself, drawn before and after,
moves one level at most, along diagonal rows of its second square and around the rim of its radial
one; nothing else on it moves. `raster_golden` moved two pages, both raster only under an unchanged list —
`function_based_shading.pdf` and `function_based_shading_cmyk.pdf` — and is re-taken here. `t2-turn_path` under one clock hold, this tree then HEAD:
33 figures banded, 33 judged, 0 outside on each, this tree over HEAD 0.941 to 1.095, median 1.005.
