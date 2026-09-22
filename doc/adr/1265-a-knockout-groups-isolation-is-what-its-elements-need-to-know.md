# 1265 — A knockout group's isolation is what its elements need to know

Status: accepted. Session 1214.
Amends: ADR 1256 §4, which named `inside_knockout`'s inability to tell the two kinds apart and
left the third construction refused inside both; ADR 1243 §2 and ADR 1170, each of which recorded
the same missing distinction from its own clause's side.
Depends on: ADR 0327 (the own-backdrop construction), ADR 1000 (the three constructions),
ADR 0237 (`Command::Group`'s `isolated`), ADR 0307 (NOTE 6's transparent backdrop).
Context: `crates/pdf-model/src/content.rs`, `crates/pdf-model/src/content/transparency.rs`,
`crates/pdf-model/src/content/pattern.rs`, `crates/pdf-model/src/content/overprint.rs`.
Clauses: ISO 32000-2 §11.4.5, §11.4.6, §11.6.7, §11.7.4.4.

`§N` is ISO 32000-2 and nothing else.

## 1. One sentence of §11.4.6 that three rows were each half-reading

> A knockout group may be isolated or non-isolated; that is, isolated and knockout are independent
> attributes. A nonisolated knockout group composites its topmost enclosing element with the
> group's backdrop. An isolated knockout group composites the element with a transparent backdrop.

`Interpreter::inside_knockout` was a `bool`. It could say *that* content was inside a knockout
group and not *which backdrop that group hands its elements*, so every site that had to state what
an element composites onto answered the same way for both:

- §11.4.6 (`implicit_knockout_group`): the own-backdrop construction refused inside any knockout
  group.
- §11.6.7 (`pattern.rs`): `isolated = self.inside_knockout || …`, so a tiling cell inside any
  knockout group took the isolated construction, and a report named the departure.
- §11.7.4.4 (`overprint.rs`): `implicit_group_statable` refused the first bullet's group to a
  direct element of a non-isolated knockout group, and the parts were painted flat.

The field is now `Interpreter::enclosing_knockout: Option<KnockoutKind>` — `Isolated`,
`NonIsolated`, or `None` where the content is not a direct element of a knockout group. It
replaces `transparent_initial_backdrop` as well, which was the same question asked of one kind:
`Some(Isolated)` *is* that flag.

## 2. Direct element, not descendant

NOTE 6 and the clause's own opening sentence are about a **direct element**:

> In a knockout group, each individual element shall be composited with the group's initial
> backdrop rather than with the stack of preceding elements in the group.

`transparent_initial_backdrop` was already scoped that way and `inside_knockout` was not: it stayed
true for everything a non-knockout group nested in a knockout group opened. That over-approximation
refused constructions at content the clause does not reach — a `B` inside an ordinary form inside a
knockout group is an element of the *form*, and the own-backdrop construction is exactly right for
it. Merging the two fields settles it as *direct*, which is also what §11.4.4's NOTE 5 asks when it
tests a group against "the same knockout attribute as its parent group", and what a soft mask's
group is not an element of at all.

## 3. What each site answers now

- **`implicit_knockout_group`** refuses the own-backdrop construction only under `Some(Isolated)`,
  where the enclosing group draws its elements on transparency and this command — seeded from its
  immediate backdrop — would take the accumulation. Under `Some(NonIsolated)` the enclosing group
  keeps its initial backdrop and hands each element a private clone of it (ADR 1256), which is what
  the command is seeded from, so NOTE 6 is met rather than refused.
- **`pattern.rs`** states §11.6.7's group truthfully: isolated where NOTE 6 gives the cell a
  transparent initial backdrop or where the cell's own NOTE 1 makes the isolated construction
  exact, and the clause's non-isolated group otherwise. The report is gone because there is no
  longer a substitution to name.
- **`overprint.rs`** builds §11.7.4.4's first-bullet group in either kind, with `isolated` stating
  which backdrop NOTE 6 gives it. `implicit_group_statable` and its two refusals are gone.
- **`run_transparency_group`** forces `isolated: true` on a nested group only under
  `Some(Isolated)`; under `Some(NonIsolated)` Table 145's `/I` is stated as the file wrote it.
  `knockout_construction` gains the guard that makes that safe: a group holding an element that is
  itself a non-isolated group cannot take either construction that draws its elements on
  transparency, whichever route reaches them.

## 4. The fixtures, and what each is calibrated against

Each is derived from the clause and each fails when the boolean is planted back (trap 13).

- `tiling.rs::a_cell_takes_the_backdrop_its_enclosing_knockout_group_has`: a Multiply cell over a
  grey page, painted under Screen inside a knockout group. Non-isolated draws 160 —
  `128 × 128 ÷ 255` for the cell against the page, then `128 + 64 − 128 × 64 ÷ 255` — and isolated
  draws 128, because §11.3.6 leaves a blend mode nothing to do on a transparent backdrop. Planting
  `is_some()` back draws 192 in the non-isolated arm.
- `overprint.rs::a_pairs_group_takes_the_backdrop_its_enclosing_knockout_group_has`: §11.7.4.4's
  first bullet inside each kind, asserted against a page painting the colour the clause names at
  the same alpha. Non-isolated keeps the backdrop's other three components (`0.9 0 0 0.5`);
  isolated has no backdrop to keep (`0.9 0 0 0`).
- `transparency_groups.rs::a_group_inside_an_isolated_knockout_group_takes_the_transparency_note_6_gives_it`
  draws its third arm rather than reporting it: a non-isolated group inside a non-isolated knockout
  group over the white page is Multiply of white with red, which is red — the same pixel the
  isolated arm draws, and NOTE 6 is why. Its second arm, `/K false`, draws black and is the control.
- `transparency::tests` asks `implicit_knockout_group` for both kinds directly: `Some(Isolated)`
  refuses and `Some(NonIsolated)` returns the own-backdrop construction.

## 5. What is left

An enclosing knockout group that is **isolated** still cannot hand a nested group the outer group's
initial backdrop, because there an element's backdrop is the accumulation the shape has been erased
from. That is the whole of §11.4.6's NOTE 6 residue now, and nothing emits a `Command::Group` with
`isolated: false` into such a group — the three sites above all state `true` there — so the
position is reached only by a file whose own form group says `/I false` inside an isolated knockout
group, which `knockout_construction`'s guard keeps out of the two transparency routes.
