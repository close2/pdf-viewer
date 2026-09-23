# 1305 — A knockout group under a mode of its own takes §11.4.4's result step

Status: accepted and **built**. Session 1234.
Amends: ADR 0327 (the own-backdrop construction's condition that the `Do` be Normal).
Depends on: ADR 0237, ADR 1107, ADR 1265, ADR 1301.
Context: `crates/render-cpu/src/lib.rs` (`knockout_on_backdrop`),
`crates/pdf-model/src/content/transparency.rs` (`knockout_construction`),
`crates/pdf-render/src/display_list.rs` (`Command::Group`'s `knockout`).
Clauses: ISO 32000-2 §11.3.5, §11.3.6, §11.4.4, §11.4.6, §11.4.8.

`§N` is ISO 32000-2 and nothing else.

## Decision

A non-isolated knockout group whose elements blend under modes no construction moves, and whose
`Do` composites under a mode other than Normal, is drawn rather than drawn flat with a report.

§11.4.4 says what the group's result is composited with and under which function:

> The result of applying the group compositing function shall then be treated as if it were a
> single object, which in turn is composited with the group's backdrop according to the formulas
> defi ned in this subclause. In those formulas, the colour, shape, and alpha ( C, 𝑓 , and α )
> calculated by the group compositing function shall be used, respectively, as the source colour
> 𝐶𝑠 , the object shape 𝑓 j , and the object alpha α j .

§11.4.8 states the result step `C = Cn + (Cn − C0) × (α0 ÷ αgn − α0)`, `α = αgn` once for every
kind of group, knockout included, and its knockout recurrence for the group alpha is stage b) with
a transparent initial backdrop, `αgᵢ = (1 − fsᵢ) × αgᵢ₋₁ + αsᵢ`, because Table 140's group alpha
excludes the backdrop. So `knockout_on_backdrop` runs each element a second time onto
transparency and averages it by the same stated shape into a second accumulation, and hands
`blend::remove_backdrop` both; the caller then composites the result once under the mode, the
path a non-knockout group already takes (ADR 1107). Under a Normal `Do` nothing changes: the
collapse `(1 − w) × P + w × buffer` is exact and the second accumulation is not kept.

`pdf-model`'s condition is the one line that let the construction happen only under Normal. The
mode-to-the-`Do` route (ADR 1009) still needs a Normal `Do`, because there is no Normal left there
for a mode to move into.

## The fixture

`a_knockout_group_under_a_mode_of_its_own_is_composited_as_one_object`: page `(1, 1, ½)`, cyan
opaque Normal under magenta at `ca ½` Multiply, `Do` under Multiply. By hand: stage a) gives
`(1, ½, ½)`, the group alpha where magenta paints is ½ (cyan knocked out of it too), the result step
gives `(1, 0, ½)` at ½, and §11.3.6 under Multiply gives `(255, 128, 96)`. The Normal collapse draws
`(255, 128, 128)`; the accumulation composited without the step draws `(255, 128, 64)`, which is
what the fixture draws with the second accumulation planted away.

## Consequences

- §11.4.6's first residue is gone; the row stays `partial` for ADR 1306's.
- `render-gpu` and `render-raster` refuse the own-backdrop construction by name whatever the mode,
  as they did; the frame goes to the oracle.
- Cost: one more `encode` of each element and one surface-sized buffer, only for this combination.
  `raster_golden` holds 974: no tracked first page states it.
