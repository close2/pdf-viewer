# 1307 — A non-isolated group under a blend of its own is drawn in `raster/`, from the clause alone

Status: accepted. Session 1235.
Acts on: `doc/questions/A76`'s independence rule, applied a second time to the same family.
Builds on: raster's ADR 0019 (the seeded layer and the interpolation under Normal), ADR 1295.
Amends: raster's ADR 0019 in one respect — its refusal of a non-Normal group blend is deleted.
Closes: `doc/QUORRA_FEEDBACK.md` section 16's excluded case, and section 49's witness.
Clauses: ISO 32000-2 §11.4.4 (Tables 139 and 140, NOTEs 3 and 4), §11.4.5, §11.3.5, §11.3.6,
§11.3.7.3, §11.7.4.3's last paragraph.

## 1. Independence

This round did not open, grep or read anything under `crates/render-cpu/`. The construction,
the WGSL and every expected value in `raster-gpu/tests/non_isolated_blended_groups.rs` and in
`render-raster/tests/headless_quorra.rs::cpu_and_quorra_agree_on_a_non_isolated_group_that_blends`
come from §11.4.4's recurrence and Result step, §11.3.6's formula and §11.3.5's blend
functions. `render-cpu` enters as a black box in that test's `compare` and in the corpus run.
One thing about the other reading was seen, and after the construction here was built and
measured: the ledger row for §11.4.4, which this round edits, says in a sentence that
`render-cpu` also keeps NOTE 4's second accumulator by running the elements again onto
transparency. The two readings chose the same method from the same NOTE; neither's numbers
entered the other.

## 2. The arithmetic

The Result step is `C = Cn + (Cn − C0) × (α0/αgn − α0)` with `α = αgn`. Multiplied through by
`αg` and with `αn = Union(α0, αg)` it is, premultiplied,

```text
αg × C = E(B) − (1 − αg) × B
```

`E(B)` is the seeded layer ADR 0019 already draws and `B` the backdrop the composite already
reads, so the only missing quantity is Table 140's group alpha `αg` — NOTE 4's "two sets of
variables". §11.3.7.3 accumulates alpha by the union whatever the blend mode and whatever the
initial backdrop, so **the elements drawn a second time onto transparency have `αg` as their
alpha**. The recovered group `(αg × C, αg)` is then composited by §11.3.6 with `B(Cb, Cs)` the
group's own `/BM`, all sixteen modes; the clip meets it as it meets an isolated group, since
`αg` is the group's shape where every opacity is one (raster's ADR 0074).

## 3. What was built

- `raster-scene`: `NonIsolatedReason::GroupBlendNotNormal` is deleted with its check — a refusal
  nothing reaches is a lie. `GroupSpec::isolated` states both constructions and the two
  knockout conditions that remain.
- `raster-gpu`: `encode_group` plans the elements twice for this case, the second time with
  `group_alpha_walk` set, under which every nested group is drawn isolated and under Normal —
  alpha-identical and one layer, so nesting costs a second walk per level, never a doubling
  per level. A soft mask's plan is always drawn in colour. `ChildOp::group_alpha` names the
  second plan; `composite.wgsl` binds it at slot 5 and `non_isolated == 2` takes
  `group_result` before §11.3.6. No division: the clamp to `[0, αg]` only absorbs the two
  eight-bit rasters' rounding.
- Cost: a second encode and raster of the group's elements, for this case only.

## 4. Measured

- The derivation against the clause transcribed, 50 000 random nested groups per mode, all
  sixteen: worst premultiplied deviation 1.3 × 10⁻⁵ (f32).
- On the device, sixteen modes over an opaque and a translucent backdrop at group alpha 1 and
  ½: 64 cases, worst 1 level of 255; a nested non-isolated Screen group inside a Multiply one
  within 3.
- `issue12798_page1_reduced.pdf` leaves `REFUSED_BEFORE_THE_SCENE` and agrees with the CPU
  oracle: mean 0.0053, worst tile 0.82 at 100%; 0.0124 and 1.70 at 200%. No further refusal
  stood behind this one.
