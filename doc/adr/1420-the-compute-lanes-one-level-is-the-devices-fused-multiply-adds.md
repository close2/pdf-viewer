# 1420 — The compute lane's one level is the device's fused multiply-adds, and it is held there

Status: accepted. Session 1291. Answers `doc/QUORRA_FEEDBACK.md` section 57's ask and ADR 1407
section 1's "which operation rounds differently was not isolated"; restates raster's ADR 0094 for
glyphs; keeps ADR 0005's rounding. Supersedes nothing.
Context: ISO 32000-2 §10.7.4; raster's ADR 0082 (one level between a device and the processor),
ADR 0080 (the compute lane is `fill_mask`'s statements); WGSL section 15.7.5.
Code: `raster/crates/raster-gpu/src/compute.rs` (module comment only).
Test: `raster/crates/raster-gpu/tests/compute_lane.rs`'s
`the_compute_lane_is_within_one_level_of_the_cpu_lane_on_glyphs`, unchanged in what it asserts.

**I did not open `crates/render-cpu/`.** The comparison is raster's CPU lane against its own
compute lane.

## What the clause fixes and what it leaves

§10.7.4 decides which pixels a shape paints; it says nothing of how a fractional coverage becomes a
level. That is ADR 0005's documented choice, and both lanes make it the same way: round half up of
one `f32` (`(cov · 255).round()` against `floor(cov · 255 + 0.5)`, which agree wherever the product
is not `0.5 − 2⁻²⁵` — checked by stepping sixty-four ulps each side of every half-level — a value
neither fixture reaches).

## The measurement

The glyph fixture (300 placements of a rotated `v` and a `w` on half-pixel baselines), both lanes on
each adapter, the atlas and the hybrid off, then scratch variants of the processor's arithmetic in
an export (removed):

| processor variant | RADV pixels | llvmpipe pixels |
|---|---|---|
| as shipped | 6 | 2 |
| `hypot` → `√(w² + h²)` | 6 | 2 |
| division by a reciprocal | 7 | 3 |
| device transform as two nested fused multiply-adds | 3 | 1 |
| trapezoid deposit fused alone (the two halves gone on RADV, made on llvmpipe) | 4 | 4 |
| transform, edge interpolation and deposit fused | 1 | 3 |

Every differing pixel is one level. The two drivers' compute lanes differ from **each other** on
four pixels. So the operation is not the rounding but the multiply-adds before it: the transform
`a·x + c·y + e` accounts for the two pixels both drivers share, and the deposit's
`row[cell] += d · (1 − frac)` fused accounts for RADV's two pixels at exactly one half — the 127
against 128 of section 57 — which llvmpipe does not fuse. WGSL section 15.7.5 permits an
implementation to reassociate and to fuse, and section 17.5.32 lets even the `fma` built-in round
twice, so no WGSL program fixes these bits, and no processor arithmetic matches two drivers that
disagree. A coverage of exactly one half sits on a level's boundary (`255 / 2`), so an ulp of either
sign moves it.

## Decision

Held at one level, with this reason in the test's comment and in `compute.rs`'s module comment.
Pre-quantising the coverage to a fixed-point grid before rounding was considered and not built: it
moves the boundary a fused ulp can cross from the half-levels to the grid's half-steps, which a
generic coverage meets more often, and it would change the CPU lane's bytes on every page for a
tie the device can still produce upstream. Exact agreement would need both lanes in integer
arithmetic from the flattening on — a new rasteriser, not a rounding rule.
