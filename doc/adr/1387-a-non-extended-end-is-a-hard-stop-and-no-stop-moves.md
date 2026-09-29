# 1387 — A non-extended end is a hard stop, and no other stop of the ramp moves

Status: accepted. Session 1275.
Context: `crates/render-cpu/src/shading.rs` (`stops`), `crates/render-cpu/tests/stripe_rows.rs`,
`crates/render-gpu/src/shading.rs` (`stops`, `END_NUDGE`), `crates/render-gpu/tests/headless_gpu.rs`.
Clauses: ISO 32000-2 §8.7.4.5.3, §8.7.4.5.4, §7.10.4, §10.7.3.

## What was measured

The brief handed on a diagnosis that the CPU oracle places `issue10572.pdf`'s stripe rows wrong
(0, 3, 7, 13 rows off over the page at 1×, 2×, 4×, 8×) and guessed a precision loss in the gradient
parameter. It is not precision. The column fixture (`stripe_rows.rs`: the page's axis, domain,
matrix and twenty-four hard stripes) put the boundaries exactly on their rows under
`/Extend [true true]` at every scale, and off by up to 3 rows at 8× under `[false false]`, 5 under
`[true false]`, and by a different amount again under `[false true]`. The variable was `/Extend`.

`stops` cut a non-extended end by placing a transparent stop at the ramp's end and **compressing every
ramp stop into `[0.0005, 0.9995]`**, so that the fade to transparent happened inside the ramp. That
moves a stop at `s` by `0.0005 − 0.001 s` of the axis: zero at the middle, which is `t = 0` on this
page and why 1× showed nothing, and 0.45 units at `t = 3` on an 1 800-unit axis — 3.6 rows at 8×.
The module comment's claim that the fade was below a pixel on any page was wrong on the same page: 0.9 units is
7 rows at 8×.

§8.7.4.5.3's Table 79 makes the parameter a linear function of the projection onto the axis alone —
"[t]he variable is considered to vary linearly between these two values as the colour gradient varies
between the starting and ending points of the axis" — so a stop moved along the ramp is a colour moved
along the page. §10.7.3 does not license it: smoothness bounds the colour error of a piecewise-linear
approximation to the function, and this was a whole-range error at displaced rows.

## Decision

The ramp's stops are handed to `tiny-skia` at their own positions. A non-extended end adds a
transparent stop *at the same position* as the ramp's end stop — a hard stop — and `Pad` repeats the
transparency beyond it, which is §8.7.4.5.3's "t is undefined and the point shall be left unpainted".
`tiny-skia` 0.12 keeps hard stops at 0 and 1 under `Pad`: with non-uniform stops it skips the clamp
and evaluates its general gradient stage on the unclamped parameter (`shaders/gradient.rs`), and a
ramp with an end stop and a transparent stop at one position is never uniform. The edge is decided at
each pixel's centre, as every other evaluated shading here is.

`stripe_rows.rs` holds both halves at 1×, 2×, 4× and 8× against rows computed from the file's numbers
in `f64`: every boundary under all four `/Extend` pairs, and a shortened axis whose two cut ends each
fall on their row. Planted away (the compression restored), both fail: a boundary one row off at 2×,
and a faded row at the cut at 8×.

## What else was checked

- **Radial** shadings that are not §8.7.4.5.4's cone go through the same `stops` into
  `RadialGradient`, so they shared the defect and share the fix. Cones go through
  `pdf_render::RadialRaster` and a `/Background` through `ShadingRaster`, both evaluated per pixel
  and untouched by it.
- **Function-based** (type 1) shadings do not use a ramp: `sampled_shader` samples at the device's
  grid (ADR 0408). Not this mechanism.
- **The ramp** is not where the loss was: `Ramp::sample_across` carries each `/Bounds` break as two
  stops at one exact `f32` position, which the `[true true]` case shows landing on its row at 8×.
- **`render-gpu/src/shading.rs`** had the same `CUTOFF` compression and is fixed the same way, within
  what Vello's 512-texel ramp admits: a hard stop at 0 holds, one at 1 does not (texel 511 takes the
  first stop at its offset, so `Pad` painted the end colour forever), so the far end's pair sits
  10⁻⁶ below 1 — both cuts and every stop within half a texel, which `headless_gpu.rs` holds, with
  the four `/Extend` pairs now drawing identical stripe rows.

## Cost

None measured: the stop list is two entries shorter than before at most, and the shader is the same.
