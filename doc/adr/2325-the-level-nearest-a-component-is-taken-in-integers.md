# 2325 — The level nearest a component is taken in integers

Status: accepted for integration. Session 1745, 2026-10-25, the pixels slot of batch one hundred and
fourteen. Clause: ISO 32000-2 §10.7, which leaves the conversion of a colour to the device's bits to
the device; no ledger row moves, because the rounding is this tree's documented choice (ADR 2311).
Amends ADR 2311 §2 on how its rule is computed, and keeps the rule. Context: ADRs 2310, 2311, 2324;
trap 251. Code: `raster-gpu/src/shaders/{rect,coverage,line,shading,function_lane}.wgsl`
(`nearest_level`, five textual copies), `shaders/copies.rs` (`PROMISED_COPIES`, which now holds
them). Tests: `raster-gpu/tests/an_opaque_paint_is_written_as_its_level.rs`'s
`an_opaque_grey_is_written_as_the_level_nearest_it_on_every_adapter`.

## 1. What ADR 2311's construction did on the arms' adapter

ADR 2311 §2 chose the level nearest a component's own `f32` value, half away from zero, and took
the product with 255 exactly by adding `fma(c, 255, −c·255)`'s residual to the `f32` product's
fraction. The rule is right and its test was green — on llvmpipe, which the suite's headless device
is. Trap 251 asks for the arms' adapter too. A fixture of 765 opaque greys, three per level `k` below
255 — the `f32` nearest `(k + ½) / 255` and the `f32` either side of it — written through that
construction drew every grey at its exact level on llvmpipe and **127 of the 255 middle greys a
level high on RADV** (Radeon 890M), every one between `k` = 128 and 254, while the 510 either side
were right on both. A grey whose exact product lies just under a half level is one whose `f32`
product rounds onto the half, and the residual is what carries it back below; on RADV it did not —
the fraction read exactly a half and rounded up, which is the plain `⌊255 c + ½⌋` ADR 2311 had
measured as wrong. Why the residual was lost there is the driver's compiler and is not this tree's
to establish; that it was lost is the fixture's.

## 2. The construction

The product is taken in integers, where nothing can contract or reassociate it. A component clamped
to `0..=1` is an `f32` whose 24-bit significand `m` (the implicit bit restored for a normal value)
times `2^-s` is its exact value, with `s = 150 − e` for a biased exponent `e` (149 for a subnormal).
`255 m < 2^32`, so `⌊510 c⌋ = (255 m) >> (s − 1)` exactly in a `u32` — and zero where `s − 1` is 32
or more, since `255 m` is below `2^32` — and the level nearest `c`, half away from zero, is
`(⌊510 c⌋ + 1) >> 1`. For `0.7`: `m` = 11 744 051, `s` = 24, `⌊510 c⌋` = 356, the level 178; for
`0.3`, 77; for `0.5`, the one exact half in the range, 128; for `1.0`, 255. The rule is ADR 2311's
unchanged, so every page llvmpipe drew is drawn the same, and RADV now draws it too: the fixture
reads 765 of 765 on llvmpipe, on RADV and on radeonsi's GL driver of the same card.

The five shaders that hand the target a level — the shading and function lanes (ADRs 2310, 2311)
and the three solid lanes (ADR 2324) — hold one copy each, and the copies are now a promise
`copies.rs` checks rather than a sentence each copy made about one other.

## 3. What it costs

About a dozen integer operations per component where the `fma` form had six floating-point ones;
the priced figure is ADR 2324 §5's turn gate, which drew through the same shaders.
