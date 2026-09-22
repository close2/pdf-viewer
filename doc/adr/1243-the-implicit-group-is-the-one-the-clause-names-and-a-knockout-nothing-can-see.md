# ADR 1243 — §11.6.7's implicit group is the one the clause names, and its knockout half is a rule nothing can see

Status: accepted. Session 1203.
Amends: ADR 0213, whose isolated construction for a tiling pattern's cell was right on NOTE 1's
condition and carried a second condition that has since expired.
Depends on: ADR 0237 (`Command::Group`'s `isolated`), ADR 1107 (§11.4.4's result step performed
for itself), ADR 1206 (its price), ADR 0529 (Table 77's wash inside the implicit group).
Context: `crates/pdf-model/src/content/pattern.rs`, `crates/pdf-render/src/shading.rs`,
`crates/pdf-model/tests/tiling.rs`, `crates/pdf-model/tests/transparency.rs`.
Clauses: ISO 32000-2 §11.6.7, §11.4.4, §11.4.6, §11.3.3, §11.3.6, §8.7.4.3.

`§N` is ISO 32000-2 and nothing else.

## 1. The sentence, and the two halves of it this tree had answered differently

> In both cases, the pattern definition shall be treated as if it were implicitly enclosed in a
> non-isolated transparency group: a non-knockout group for tiling patterns, a knockout group for
> shading patterns.

A tiling pattern's cell got an **isolated** group under two conditions: inside a knockout group, and
wherever the *mark's* blend mode was not Normal. A shading pattern's implicit group was recorded as
"not built", on the argument that `sh` composites its result once.

## 2. The tiling half: one of the two conditions was ADR 0237's collapse showing through

NOTE 1 makes an isolated cell exact where nothing in it blends — "the results depend only on the
colour, shape, and opacity of the pattern cell and not on those of the backdrop" — and that
condition stands. The other came from what a backend could draw: a non-isolated group was drawn by
the collapse ADR 0237 derives, which holds only where the group's own blend is Normal.

**That premise expired with ADR 1107.** `render-cpu`'s `group_buffer` performs §11.4.4's result step
for itself under any mode at the `Do` — the elements run a second time onto transparency for Table
140's group alpha, `remove_the_backdrop` removing the backdrop once — priced by ADR 1206 at +7.12%
on the one curated page that reaches it. `render-raster` refuses the combination by name at
`SceneBuilder::group` and `render-gpu` refuses every non-isolated group. So the interpreter can
state the clause and let each backend answer for itself, which is what it now does: `isolated` is
`self.inside_knockout || !any_command(&parts, &command_blends)` and nothing about the mark.

What is left reported is a *backdrop* rather than a mode, and the conjunction says so by itself: a
cell inside a knockout group whose own initial backdrop is not transparent, where §11.4.6's NOTE 6
gives the inner group "the same as that of the outer group" and none of the three constructions can
hand that over (ADR 1205 §3 reaches the same position from `implicit_knockout_group`).

## 3. The shading half: the knockout is unobservable, which is not the same as unbuilt

The implicit group of a shading pattern holds at most two elements — Table 77's wash, which the
fourth bullet puts inside it, and the shading — and **both are opaque and painted Normal**:

- the first bullet initialises the blend mode, the soft mask and the alpha constant to their
  standard default values, and `PatternInitial::augmented` admits none of the three back from
  Table 75's `/ExtGState` (it reads `/UseBlackPtComp`, `/RI`, `/SM` and §10.4.2.4's pair, and the
  comment above it says of each Table 57 entry why);
- a shading's colours carry no alpha of their own; §11.6.4.4's constant is the *mark's* and is
  folded into every colour the shading answers with, the wash among them, at the painting
  operation.

§11.3.3's formula gives an opaque Normal source `Cs` whatever backdrop it is given, so compositing
each element with the group's **initial** backdrop (§11.4.6) and with its **immediate** one
(§11.4.4) produce the same colour and the same alpha. The group's shape agrees too: the wash fills
"those portions of the area to be painted that lie outside the bounds of the shading object", so at
every point where anything paints, the topmost enclosing object's shape *is* the union.

So the knockout attribute changes no pixel of a shading pattern. The row's residue was a claim about
the clause, and the clause's own initialisation is what empties it.

## 4. What was built, and what holds it

- `compose_tiling`'s isolation condition, above.
- `tiling.rs::a_cell_that_blends_gets_the_non_isolated_group_the_clause_names`: the page's grey at
  128 of 255, the cell's Multiply element at `128 × 128 ÷ 255` = 64, the group composited once under
  the mark's Screen at `128 + 64 − 128 × 64 ÷ 255` = 160 — against 192 for the isolated cell, where
  §11.3.6 leaves a blend mode nothing to do on a transparent backdrop. Calibrated by planting the old
  condition back (trap 13): the fixture then reports "non-isolated, and an element blends with the
  backdrop it excludes" and the test fails. NOTE 1's own case is the second arm and draws 192.
- `transparency.rs::a_shading_patterns_wash_and_ramp_carry_the_marks_constant_once`: a pattern whose
  `/ExtGState` states `/ca 0.5`, `/CA 0.5` and `/BM /Multiply`, painted under `/ca 0.25`. The ramp,
  the wash and the command's blend mode are the mark's, which is §3's premise held rather than
  argued. Calibrated by planting `Shading::with_colours`' mapping of `background` away.

**No corpus page reaches either.** All 122 tiling paints in the corpus leave the three transparency
parameters at their defaults, so §11.4.4's NOTE 5 keeps the commands inline and no group is built at
all; both fixtures are hand-built and trap 8's rule is why that is said here rather than assumed.

## 5. What a later round should not re-litigate

That a shading pattern's implicit group being a *knockout* group is unbuilt work. It is a rule whose
two constructions provably coincide, on the clause's own initialisation of the parameters that could
have made them differ. A round that gives a shading pattern's implicit group a blend mode or an
alpha constant of its own would break that argument — and would be reading Table 75's `/ExtGState`
past §11.6.7's first bullet, which is the thing to argue first.
