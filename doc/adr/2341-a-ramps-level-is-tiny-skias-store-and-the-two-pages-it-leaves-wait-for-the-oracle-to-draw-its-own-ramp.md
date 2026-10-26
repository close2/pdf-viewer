# 2341 — A ramp's level is `tiny-skia`'s store, and the two pages it leaves wait for the oracle to draw its own ramp

Status: deferred. Session 1753, 2026-10-26, the oracle slot of batch one hundred and fifteen. Clause:
ISO 32000-2 §10.7, which leaves the conversion of a colour to the device's bits to the device, and
§8.7.4.5.3–§8.7.4.5.4 for what a ramp is; no ledger row moves. Context: ADRs 2310, 2311, 2324 §4,
2325, 2340. Code: `render-cpu/src/shading.rs` (`shader`, `stops`), `tiny-skia` 0.12.0's
`pipeline/highp.rs` (`unnorm`, `store_8888`).

## 1. What is left of the eleven rises

ADR 2324 §4 named eleven page-lanes whose mean error rose when the device began writing the level
nearest a component (ADR 2325): `function_based_shading.pdf` on six, `bug852992_reduced.pdf` on
three, `issue840.pdf` on two. ADR 2340 takes the first page's six. The other two pages are each an
axial shading — `bug852992_reduced.pdf`'s inside a luminosity mask, `issue840.pdf`'s under an opaque
fill — and the oracle hands an axial or radial ramp to `tiny-skia` as a gradient (`shading::stops`).
The colour at a pixel is then interpolated in `f32` inside the library and written by its high
precision store, `unnorm`: the `f32` product with 255, rounded to nearest-even. Where that product
lands on a half level that the exact value is below, the store writes the level above half the time,
and the device's ramp lane, which writes the level nearest the exact value (ADR 2310 §3), the level
below. ADR 2310 §4 met the same tie on one row of `issue14165.pdf`. No code in `render-cpu` sees the
interpolated value before the store, so nothing in this tree can round it.

## 2. The two levers, and why neither is taken now

- **The oracle draws its own ramp**: evaluate `pdf_render::Ramp::colour_at` at each device pixel's
  own `t` — the axial projection, or §8.7.4.5.4's two-circle solution — write the level nearest it
  (ADR 2340's `nearest_level`) into a grid at one cell per pixel, and hand that grid to `tiny-skia` as
  the sampled shading already is. It is a new construction of every axial and radial page the corpus
  holds, whose `t` would differ from the library's in its last bits, so it moves far more than the
  ties: a round of its own, priced on the arms, with `raster_golden` re-taken.
- **A patched `tiny-skia` whose store rounds the exact product**: one function, but it is the store
  of every highp draw, so it would move every composite whose `f32` result lands on a tie — the
  blends and partial coverage included, where the device's own blend is the hardware's and was never
  rounded this way (trap 257's shape on the oracle's side) — and it is a fork of a dependency.

## 3. When to take it

When a round owns the oracle's ramps — the first lever, which needs no one's word — or when the
project takes a fork of `tiny-skia` for another reason, at which point the second lever is priced
against the first. Until then the two pages are left as ADR 2324 §4 states them, ties of the
oracle's arithmetic that the library makes and this tree cannot reach.
