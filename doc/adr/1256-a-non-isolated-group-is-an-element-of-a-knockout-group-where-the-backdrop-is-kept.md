# 1256 — A non-isolated group is an element of a knockout group wherever the initial backdrop is kept

Status: accepted. Session 1209.
Amends: ADR 1205 §3, which named `implicit_knockout_group`'s refusal of a non-isolated element as
NOTE 6's backdrop and left it whole; and ADR 1243 §2, which recorded the same position from
§11.6.7's side.
Depends on: ADR 0327 (the own-backdrop construction), ADR 1000 (the three constructions),
ADR 1170 (§11.7.4's implicit groups), ADR 0237 (`Command::Group`'s `isolated`).
Context: `crates/pdf-model/src/content/transparency.rs`,
`crates/pdf-model/tests/transparency_groups.rs`, `crates/render-cpu/src/lib.rs`.
Clauses: ISO 32000-2 §11.4.6, §11.4.4, §11.6.7, §11.7.4.3, §11.7.4.4.

`§N` is ISO 32000-2 and nothing else.

## 1. The note, and the sentence four paragraphs above it that answers it

> NOTE 6 When a non-isolated group is nested within a knockout group, the initial backdrop of the
> inner group is the same as that of the outer group; it is not the immediate backdrop of the inner
> group. This behaviour, although perhaps unexpected, is a consequence of the group compositing
> formulas when b = 0.

This tree read that as a capability it did not have: a display list command is seeded from its
immediate backdrop, the note wants the outer group's initial one, so an element that is already a
non-isolated group was refused outright and `Unsupported::CompositedInParts` named the page.

The clause's own opening sentence is what makes the refusal unnecessary for one of the three
constructions:

> In a knockout group, each individual element shall be composited with the group's initial
> backdrop rather than with the stack of preceding elements in the group.

For a **direct element** of a knockout group there is therefore no difference between the two
backdrops the note contrasts: what the element is composited with *is* the group's initial
backdrop, and handing the inner group the one hands it the other. That is what the note's own last
sentence says — it is a consequence of `b = 0`, not an extra rule — and it is why nothing has to be
carried on the command.

## 2. Which construction can do it, and which cannot

`implicit_knockout_group` has three, and they differ exactly in whether the initial backdrop
survives to the element:

- **Elements on transparency** (nothing blends), and **the mode moved to the `Do`**: the elements
  are accumulated on a transparent buffer and the group is composited over the page afterwards.
  §11.4.4's NOTE 3 cancellation makes that exact *for the group's result* — the backdrop is
  composited in and removed again — and it says nothing about what a nested group is given. An
  inner non-isolated group there would seed from the accumulation, which is neither backdrop. So
  these two keep the refusal.
- **§11.4.6's own backdrop** (`isolated: false` beside `knockout: true`, ADR 0327): `render-cpu`'s
  `knockout_on_backdrop` retains the initial backdrop `B` and composites each element in a
  **private clone of `B`**. An element that is a non-isolated group therefore seeds `initial_backdrop`
  from `B` itself. This is NOTE 6, exactly, with no flag and no new vocabulary.

So the blanket refusal moves from the top of the function to a guard over the first two
constructions. `render-gpu` and `render-raster` refuse `isolated: false` by name already, so the
construction stays the oracle's and no other backend draws a plausible wrong picture (trap 5).

## 3. What the page is, and what it draws

§11.7.4.4's second bullet puts a `B`'s fill and stroke in a non-isolated knockout group, and
§11.6.7 makes a tiling-pattern fill whose cell blends a **non-isolated group** of its own — so the
fill is a group nested in a knockout group, which is what NOTE 6 is about. That is one of the two
routes that arrive with such an element; the other is a form XObject inside a Type 3 glyph.

`a_non_isolated_group_may_be_an_element_of_a_knockout_group` fills and strokes one square in a
single `B` with both alpha constants at 0.5, the fill an opaque blue tiling pattern under
`/BM /Multiply` and the stroke opaque green four units wide. The expected values are the clause's
rather than this tree's:

- The bullet performs the parts "with their respective prevailing alpha constants and the
  prevailing blend mode" and composites the group "using an alpha value of 1.0 and the Normal blend
  mode", which over a non-isolated group's own backdrop is the accumulation itself.
- Inside the stroke the stroke is the topmost element at shape 1.0, so NOTE 5 gives "the colour and
  opacity that result from compositing the object with the initial backdrop" — green at `CA 0.5`
  over the white page, `(127, 255, 127)`.

That is what it draws. Filling and stroking as two operators draws `(63, 191, 127)` in the same
band, which is §11.7.4.4's NOTE 2 double border; away from the stroke both draw the pattern.
Planting the old refusal back makes the fixture fail on the report, so it discriminates (trap 13).

## 4. What is left

An enclosing knockout group that is **isolated** — the two transparency constructions and
`Compose::Knockout` — still cannot hand a nested group the outer group's initial backdrop, because
there an element's backdrop is the accumulation the shape has been erased from. `inside_knockout`
does not distinguish the two kinds of enclosing group, so the third construction is still refused
inside any of them; separating them is one field beside `Interpreter::inside_knockout` and belongs
to the round that owns `content.rs`. §11.6.7's cell keeps the isolated construction inside a
knockout group for the same missing distinction (`pattern.rs`), which is why that row does not move
with this one.
