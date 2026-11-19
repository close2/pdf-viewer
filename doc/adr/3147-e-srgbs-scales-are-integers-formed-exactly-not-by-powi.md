# 3147 — e-sRGB's and e-sYCC's scales are integers formed exactly, not by `powi`

Session 2111. Status: **accepted and built.** Context: ADR 3129 (the shading filter's weight, the same fault),
`crates/pdf-colour/src/colour/extended_srgb.rs`. Sources: PIMA 7667 equations 3 and 4 (e-sRGB) and B7 and B8
(e-sYCC); IEEE 754 for the arithmetic.

## Context

`ExtendedSrgb::offset_and_scale` and `top_and_middle` formed `2^(n−2) + 2^(n−3)`, `255 × 2^(n−9)`, `2^m − 1` and
`2^(m−1)` with `2f32.powi`. ADR 3129 showed that `powi` is not one of IEEE 754's correctly rounded operations, that
Rust leaves its precision unspecified, and that Miri perturbs it: a scale a unit in the last place off moves every
component equation 5 divides by it, and the half-code margin `within_srgb` derives from it.

## Decision

Every one of these values is an integer of at most seventeen bits — the depth is at most 16 (section 4.1's 10, 12 and
16 for e-sRGB, 1 to 16 for e-sYCC) — so each is formed as a `u32` by shifts, one product and one subtraction, and
converted once; every integer below `2^24` is an `f32` exactly. No float arithmetic stands between the equation and
the value.

## Consequence

Natively `powi` of two to a small integer was already exact, so no rendered value changes and no golden line moves.
The test `the_scales_are_the_equations_integers_exactly` compares the four values exactly at every defined depth.
