# 2311 — A program's answer reaches the target as a level, and a solid paint is left to the target

Status: accepted for integration. Session 1738, 2026-10-24, the pixels slot of batch one hundred and
thirteen. Clause: ISO 32000-2 §8.7.4.5.2 (the type 1 shading's colour "at every point in the
domain"), with §10.7, which leaves the conversion of a colour to the device's bits to the device; no
ledger row moves, because the rounding is a documented choice of this tree's — nearest, half away
from zero — and not a clause's. Context: ADRs 0011, 0339, 0376, 2310. Code:
`raster-gpu/src/shaders/function_lane.wgsl` (`fs_main`, `nearest_level`) and `shading.wgsl`
(`nearest_level`, which ADR 2310's `ramp_colour` calls). Tests:
`raster-gpu/tests/a_steep_ramp_is_read_at_its_own_t.rs`'s
`a_colour_a_hair_below_a_half_level_is_the_level_below`, red with a plain `⌊255 c + ½⌋`.

## 1. The pages

Of the pages whose three lanes draw one digest — a paint's signature rather than a coverage lane's
— `issue17679.pdf` follows `issue14165.pdf` and `bug1743245.pdf` (ADR 0413's): 0.0550 of 255 at 1x
and 4x on every lane, three opaque `DeviceRGB` fills whose 2 200 differing pixels are one channel one
level low — green `0.3`, 76.5 levels in exact arithmetic and 76.500 003 in `f32`, drawn 76 where the
oracle draws 77. It is ADR 2310 §3's adapter on a solid paint: RADV exports a fragment's colour to
the eight-bit target as a half float rounded toward zero, up to an eighth of a level low above one
half and a sixteenth below, which puts `0.3` at 76.45; llvmpipe rounds to nearest and draws 77.
`function_based_shading.pdf`, 0.0369 on every lane, is the same export under §8.7.4.5.2's device
program (ADR 0376): 14 370 of its 22 339 differing pixels were the device's one level low.

## 2. What is built: the program's answer and a ramp's colour as levels

The function lane's `fs_main` rounds the program's straight answer to the level nearest it before
opacity and coverage multiply it, as ADR 2310's `ramp_colour` does — the byte the oracle's grid of
the same function holds (ADR 0339). A level survives the half-float export. **The level is the one
nearest the `f32`'s own value**, its product with 255 taken exactly through `fma`'s residual,
because the `f32` product rounds a component a hair below a half level onto it: `0.7` is 178.499 997
levels, which `⌊255 c + ½⌋` in `f32` makes 179 where the oracle and llvmpipe draw 178. On a device
whose `fma` is not fused the residual is zero and the rounding is the plain one. After it,
`function_based_shading.pdf` differs at 14 650 pixels, both signs, the program's `f32` against the
oracle's grid — a difference of evaluation, left as ADR 0376 states it.

## 3. What is not: a solid paint's colour rounded before the device multiplies it

`issue17679.pdf` goes to 0 differing pixels if `render-raster` hands a solid paint over as its level,
and this round built that and walked it twice against HEAD's six arms. The first walk rounded every
solid paint by `byte_colour`'s `f32` product: `close-path-bug.pdf`'s `0.7` grey rose 0.0060 to 0.0149
and `issue11279.pdf`'s `SoftLight` fills 0.0009 to 0.0173 at 1x. The second rounded only a colour
the target receives unchanged — the Normal blend function, full opacity, no mask — and by the exact
product: those two came back, and 30 pages of the `cpu` arm at 1x still rose, by 0.0001 to 0.005,
nine of the ten read painting solid fills and glyphs and no ramp (`bug1669099.pdf`, `issue6127.pdf`,
`XiaoBiaoSong.pdf`, `canvas.pdf` among them). The construction is the reason: a colour rounded *before* the device multiplies
it by a pixel's coverage moves every antialiased edge by up to half a level times the coverage, and
the oracle multiplies the colour as it is. A page of fills is mostly interiors and gains; a page of
type is mostly edges and loses. So solid paints keep `colour`, unrounded, and `issue17679.pdf` stays
0.0550. **The remedy left is the write itself**: rounding the premultiplied value a lane hands the
target after coverage has multiplied it, in every solid lane's shader (`rect.wgsl`, `coverage.wgsl`,
`line.wgsl`, the quad lane) — raster's own question, and record 1738 carries it to the next pixels
round.
