# 2324 — An opaque solid paint leaves the lanes as its level, and an edge is left to the blend

Status: accepted for integration. Session 1745, 2026-10-25, the pixels slot of batch one hundred and
fourteen, under the project owner's switch to code improvements and bug fixes of 2026-10-14.
Clause: ISO 32000-2 §10.7, which leaves the conversion of a colour to the device's bits to the
device, with §11.3.3's compositing; no ledger row moves, because the rounding is this tree's
documented choice (ADR 2311). Context: ADRs 0005, 0006, 0010, 2310, 2311, 2325; trap 251. Code:
`raster-gpu/src/shaders/rect.wgsl`, `coverage.wgsl` and `line.wgsl` (`fs_main` in each). Tests:
`raster-gpu/tests/an_opaque_paint_is_written_as_its_level.rs`.

## 1. The page

`issue17679.pdf` is three opaque `DeviceRGB` rectangles whose 2 200 differing pixels were one
channel one level low on every lane at 1x and 4x, 0.0550 of 255: green `0.3`, 76.500 003 levels,
drawn 76 where the oracle draws 77, because RADV exports a fragment's colour to an eight-bit
target as a half float rounded toward zero (ADR 2310 §3). ADR 2311 §3 rounded the paint before
coverage, moved every antialiased edge, took it back, and named the remedy left: round each solid
lane's write after coverage.

## 2. That remedy, built and measured: it rounds an edge twice

Built as named — `nearest_level(colour × coverage)` in all three lanes — it took `issue17679.pdf`
to zero and `issue18529.pdf` from 0.0172 to 0.0114, and on the pages ADR 2311 §3 named it moved
edges again: `close-path-bug.pdf` 0.0060 to 0.0062 on the `cpu` lane at 1x, `canvas.pdf` 0.0133 to
0.0135, every moved pixel an edge. The arithmetic says why. Over an opaque backdrop the blend
makes a pixel `s + d (1 − a)`, and the oracle's compositing (`render-cpu`, tiny-skia's
high-precision pipeline) evaluates that in `f32` and rounds once. A lane that rounds `s` and its
alpha `a` before the blend rounds twice: over white the pixel becomes `⌊255 s⌉ + 255 − ⌊255 a⌉`
where it should be `⌊255 s + 255 − 255 a⌉`, and those differ by a level wherever the two fractions
fall either side of a half — a grey of 0.5 at 76.6 levels of coverage is 216.7, which the double
rounding draws 216. Rounding *after* coverage is still rounding before the blend; a level is owed
once, where the pixel's value is final.

A second construction quantised the coverage to a level first, as the oracle's eight-bit masks
hold it. It brought `canvas.pdf` down to 0.0128 and raised `issue6127.pdf` from 0.0125 to 0.0153:
that page's hairlines reach the oracle as sub-pixel bands whose coverage is an `f32` opacity, not a
mask level, so the construction matched one of the oracle's paths and departed from another. A rule
read off which path the oracle happens to take is the curve-fitting principle 5 forbids; it is not
built.

## 3. What is built: the write is the level where it is the pixel

Where the source is opaque — `colour.a × coverage` is 1 — the blend keeps none of the backdrop, so
the fragment's colour *is* the pixel, and the shader hands it over as the level nearest it
(`nearest_level`, ADR 2325): a level survives the half-float export. Where the source is not opaque
the shader hands over the source as it is and the blend decides the pixel from it and the backdrop,
one rounding, as the oracle composites. So every antialiased edge, every mark under a constant
alpha below 1 and every pixel of a soft mask below 1 is written exactly as before, byte for byte,
and the interiors of opaque solid marks move to their level.

The pipelines the three lanes draw under (`pipeline/spec.rs`): **over** keeps `d (1 − 1) = 0` of
the backdrop; **add**, knockout's deposit (ADR 0010), follows an erase by the same shape, which at a
shape of 1 left nothing; **destination-over**, §11.7.4.3's special overprinting mode (ADR 1295),
scales an opaque source by the backdrop's `1 − αb`, so there the level is rounded before a product —
at most half a level times `1 − αb`, on a backdrop that is not already opaque. That case is named
rather than given a shader of its own; the arms below carry the overprinting pages.

**What the adapter does at an edge is its own.** llvmpipe's blend takes the source at eight bits
before blending, so it draws the 0.5-at-76.6 edge 216 whatever the shader hands it; RADV's blend
carries the source's fraction and draws 217. Both are inside ADR 0006's bound, and the second test
holds each adapter to what it does, so a shader that rounded the edge would fail it on RADV.

## 4. The pixels

Isolated first on the marks (trap 213): every one of `issue17679.pdf`'s 2 200 pixels lies inside one
of its opaque rectangles, 224 of them where two abut, one channel one level low; and the fixture of
765 one-pixel opaque rectangles is the smallest mark there is. After this construction, on the arms'
adapter, the page differs at 0 pixels of 10 000 on both coverage lanes. The pages whose edges ADR
2311 §3 and §2 above moved — `close-path-bug.pdf`, `bug1669099.pdf`, `issue6127.pdf`, `canvas.pdf`,
`XiaoBiaoSong.pdf`, `issue11279.pdf` — move no pixel at all. `issue18529.pdf` moves 92 pixels, 64
nearer the oracle and 28 further, the mean 0.0172 to 0.0117: its red stroke is drawn under a
luminosity mask the shading lane draws, and the mask's levels move by ADR 2325 at half-level ties.

Against HEAD's six arms, page by page, every arm exit 0: 18 pages moved on each 1x arm and 25 on
each 4x arm, 129 page-lanes; the mean error fell on 72, held to four decimals on 46 and rose on 11.
The device lane's sum of page means fell on every arm — the `cpu` arm 50.0442 to 49.8095 at 1x and
16.4326 to 16.1923 at 4x. The largest falls are opaque fills: `issue17679.pdf` and
`issue17679_2.pdf` 0.0550 to 0, `issue9243.pdf` 0.0373 to 0.0095, `calgray.pdf` 0.0293 to 0.0080,
`tiling_patterns_variations.pdf` 0.0328 to 0.0125, `issue13520.pdf` 0.1072 to 0.0695 at 1x.

**The 11 rises are three pages, and each is ADR 2311's rule reaching RADV**, not this construction
moving an edge: `function_based_shading.pdf` on all six arms (0.0285 to 0.0314 at 1x, 0.0080 to
0.0087 at 4x), `bug852992_reduced.pdf` on the three 1x arms (0.0191 to 0.0196) and `issue840.pdf` on
two (+0.0001). On llvmpipe, where the `fma` residual held, each draws the same bytes before and
after (ADR 2325 changes how the rule is computed, not the rule). On RADV the pixels that moved are,
all but 138 of 2 723, 10 of 120 and 1 of 33, the bytes llvmpipe draws: a level whose `f32` product
with 255 is exactly a half and whose exact value lies below it, drawn at the level below where the
oracle's `f32` breaks it upwards. The function lane's program answer and the shading lane's mask on
the first two, an opaque fill on the third. They are ties of the oracle's arithmetic in the sense of
ADR 2311's `issue18529` — whose three rises in record 1738 fall here, 0.0172 to 0.0117 — named page
by page and left as the rule states them.

## 5. The turn

`t2-turn_path` under one `--clock` hold, this tree then HEAD's, load 4.7 to 4.3: 33 figures banded
on each, 24 and 21 judged, 0 outside on either. The 18 figures judged in both read mine over HEAD
0.949 to 1.104; over all 33 the median is 1.018. The turn figures did not move outside their bands,
and the integer rounding's price is not separable from the hold's noise at this size.
