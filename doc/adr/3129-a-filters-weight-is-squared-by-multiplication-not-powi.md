# 3129 — The shading filter's weight is squared by one multiplication, not by `powi`

Session 2107. Status: **accepted and built.** Context: ADR 3024 (a shading's `/AntiAlias` filter), ADR 3074, ADR
3094, `pdf_render::shading::box_down`. Clause: ISO 32000-2 §8.7.4.3 (`/AntiAlias`); IEEE 754 for the arithmetic.

## What failed

CI's `miri (pdf-render)` job failed
`shading::tests::an_anti_aliased_parametric_mesh_is_filtered_and_one_with_a_background_is_not`: column 0, half inside
the mesh, had alpha 127 where 128 is asserted. Natively it passes.

## The cause, proved

`box_down` weighted each block by `1.0 / f32::from(FILTER_SAMPLES).powi(2)`. `powi` is not one of IEEE 754's
correctly rounded operations, Rust documents its precision as unspecified, and Miri now perturbs its result by a few
units in the last place, drawn from the run's seed. A weight a unit under 1/16 turns eight covered samples' 0.5 into
0.49999997, so `(0.5 × 255).round()` — 127.5 rounded half away from zero to 128 — becomes 127.

Reproduced here: `MIRIFLAGS=-Zmiri-many-seeds=0..32 cargo +nightly miri test -p pdf-render --lib
an_anti_aliased_parametric_mesh_is_filtered` failed at seed 19 (and at seed 45 of `0..64`) with CI's
`(127, 255, 0)`; the default seed passes, which is why a plain local Miri run did not see it.

## Decision

**The code, not the test.** The test's 128 is exact arithmetic: half of sixteen samples, at full alpha, is 127.5, and
rounding is the code's own `round`. The weight is now `1.0 / (n × n)` with `n = f32::from(FILTER_SAMPLES)`: one IEEE
multiplication, exact for 4 × 4, and one division, exact for 1/16. With it, both anti-aliased tests pass under Miri
on all 64 seeds of `0..64`. Loosening the assertion to ±1 would have kept a weight that is not what the filter states
— the mean of `FILTER_SAMPLES`² samples — on any platform whose `powi` is not exact.

## Cost

None measured: the weight is computed once per call.
