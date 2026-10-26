# 2350 — A translucent ramp reaches the blend as it is, and destination-over keeps the level

Status: accepted for integration. Session 1758, 2026-10-26, the pixels slot of batch one hundred and
fifteen, under the project owner's switch to code improvements and bug fixes of 2026-10-14. Clauses:
ISO 32000-2 §8.7.4.5.3 (an axial shading's colour at a point), §11.3.3 (the compositing formula),
§11.7.4.3 (the special overprinting blend mode), with §10.7, which leaves the conversion of a colour
to the device's bits to the device; no ledger row moves, because the rounding is this tree's
documented choice (ADR 2311). Narrows ADR 2310's rounding of a ramp's colour to an opaque paint, and
settles ADR 2324 §3's open cost. Context: ADRs 1295, 2286, 2297, 2310, 2311, 2324, 2325, 2351; traps
213, 232, 251, 257. Code: `raster-gpu/src/shaders/shading.wgsl` (`fs_main`, `ramp_interpolated`).
Tests: `raster-gpu/tests/a_translucent_ramp_is_rounded_once.rs`, red on the tree before (408 of
1 020 pixels off on RADV); `raster-gpu/tests/an_opaque_paint_under_the_backdrop.rs`, red with the
level withheld under destination-over (381 of 765 off on RADV).

## 1. The ranking

Against HEAD's six arms the head of the ranking is the glyph quantum's (ADR 0498), so the 1x arms
were taken again with the quantum off, as ADR 2286 §1 ranks: the `cpu` arm's sum of page means is
8.7481 there against 49.8050 shipped. Its head is three pages already named — `issue2177` (ADRs 1435,
1443), `issue18823` and `bug1802506` (`doc/todo/11`) — and then `knockout_groups_test` (0.1883 on
the `gpu` lane, 0.1292 on `cpu` and `compute`) and `issue7821` (0.1296 on all three). Below them the
pages are ambiguous-bucket or glyph-edge residues already named (`issue269_1`, `bug1743245`,
`issue15716`). Each page was isolated on its own commands, at full size (trap 213).

## 2. `issue7821.pdf`: the device rounded a translucent ramp twice

Command 0 alone, an axial shading whose stops carry alpha 0.849991, holds 0.1305 of the page's
0.1296: 3 605 pixels one level off. §8.7.4.5.3 gives the pixel the ramp's colour at its centre's
`t`, and §11.3.3 over the opaque page makes it `(1 − α)·cb + α·cs`. ADR 2310 had the shading lane
round the ramp's straight colour and alpha to a level before the shading's alpha and the pixel's
coverage multiplied it, which is trap 257's construction: rounded once in the shader and again at
the blend, off by up to half a level times α before the blend's own rounding. The oracle's gradient
(`render-cpu`'s `tiny_skia::LinearGradient`) interpolates the stops in `f32` and rounds once.

**What is built**: `fs_main` takes the level of an opaque paint, as ADR 2310 did — the interior of
an opaque ramp is the pixel, and RADV's half-float export would take it a level down (51 of
`issue14165.pdf`'s rows) — and hands a paint below full alpha over as it is, for the blend to round
once. Mesh paints are levels already and draw the same bytes. Handing every paint over unrounded,
ADR 2324's own rule, was measured first: on RADV it took `issue14165.pdf`'s partly covered last row
and column a level down (0.0002 to 0.0011), the half-float export again, where ADR 2310's level times
the coverage had drawn what the oracle draws; ADR 2351 prices the remedy for that export and defers
it. The function lane keeps ADR 2311's rule, which rounds a program's answer at any alpha; whether a
translucent function shading reaches the corpus's first pages was not counted.

## 3. `knockout_groups_test.pdf`: an oracle finding, handed to the oracle's round

Group 395 alone — isolated, not knockout, alpha 1 — differs at 69 696 pixels, every one the blue of
its opaque `(0.91, 0.85, 0.7)` background: the oracle 179, the device 178, 0.0446 of the page's
0.1292 on the `cpu` lane at 1x with the quantum off. `0.7` as an `f32` is 178.499 997 levels and ADR
2311's rule draws 178; so does the oracle where the same colour is a top-level fill
(`close-path-bug.pdf`'s grey, 178 on both backends). Inside an isolated group's buffer the oracle
draws 179, the `f32` product with 255 rounded half up. The device is right by the tree's rule and
the oracle disagrees with itself, so nothing is built here; the finding goes to the round that owns
`render-cpu`'s level (batch 115's slot 2). The page's other pairs are translucent marks composited
over that background, and they move with it.

## 4. Destination-over: the cost is accepted, and the fixture states both halves

ADR 2324 §3 left a cost named: under §11.7.4.3's mode with every channel kept, the lanes'
`(ONE_MINUS_DST_ALPHA, ONE)` blend scales an opaque source by `1 − αb`, so its level is rounded
before a product. A census over the `cpu` arm's 968 first pages, at the scene builder in a scratch
copy, printed every fill and group under the mode: two fills, both white `(1, 1, 1)` at alpha
0.300003 — no opaque source and no colour that is not a level — and no group. No arm page can move
on it, which is why none did (trap 232). The fixture is `an_opaque_paint_under_the_backdrop.rs`:
765 near-half greys over black backdrops of nine alphas. Handing the source over unrounded under
the mode alone, built and measured: on RADV the empty backdrop's 765 greys went from 0 to 381 off
their level — the half-float export, as `issue17679.pdf` was — while the translucent bands improved
(500 to 48 off at `αb` = 1/255, 134 to 9 at 128/255); on llvmpipe, whose blend takes the source at
eight bits, the empty band went from 0 to 128 off and every translucent band but the faintest got
worse (126 to 189 off at 128/255). So the
level is kept: it is exact where the backdrop is empty, which is what an isolated group's first mark
meets, and it costs at most half a level times `1 − αb` where the backdrop is translucent. The test
holds the first exactly and the second within one level, on every adapter present.

## 5. The pixels

Against HEAD's six arms by page, every arm exit 0 and agreeing on every page it compared: four
pages moved on each arm, 24 page-lanes; the mean error fell on 18 and rose on 6. The device lane's
sum of page means fell on every arm — `cpu` 49.8050 to 49.7206 at 1x and 16.1914 to 16.1017 at 4x,
`gpu` 50.6750 to 50.5909 and 18.2742 to 18.1841, `compute` 8.9923 to 8.9041 and 5.1761 to 5.0851.
The falls: `issue7821.pdf` 0.3062 to 0.2456 on the `cpu` arm at 1x (0.1296 to 0.0652 on `compute`),
`issue18956.pdf` 0.0241 to 0.0007, `issue17069.pdf` 0.0173 to 0.0164, on every arm.

**The six rises are one page, `alphatrans.pdf`**, by 0.0005 to 0.0009 (0.0498 to 0.0503 on the
`cpu` arm at 1x), and they are not ties. Isolated on its 90 commands: command 89, an axial ramp
from red to blue at alpha 0.5, drawn alone over the white page falls 0.0131 to 0.0015 — the
construction this ADR builds, working; the page rises only where that ramp lies over the knockout
groups 86 and 87 and image 88, where of 27 464 pixels that moved 14 477 came nearer the oracle and
10 281 went further. The oracle hands that ramp to `tiny-skia` as a gradient, whose store breaks a
half-level tie its own way (ADR 2341), and this round did not isolate the backdrop's part further;
the rise is named here as that page's, and the half-float remedy that would change it is ADR 2351's.
Looked at (trap 1): `issue7821.pdf` and `alphatrans.pdf`, both drawn by the oracle and the device
lane at 1x. Device-lane goldens: none moved, none re-taken; raster's suite 777 of 777 with the two
new tests, which draw on llvmpipe, RADV and radeonsi.
