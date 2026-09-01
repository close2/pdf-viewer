# ADR 0778 — The tie-breaker was doing the deciding

Status: accepted, 2026-09-01. Session 851, a clause round on the `owed` reading list.

`--bin owed`'s list is the `partial` rows whose every stated term this tree already names. Ordered
by when each note was last written, §8.7.4.5.7 and §8.7.4.5.8 — the Coons and tensor-product patch
meshes — sit near the top: two of the three oldest notes in the whole list, both saying the same
short thing, that what keeps them `partial` is a tessellation whose fineness is `PATCH_STEPS`
rather than §10.7.3's smoothness tolerance.

That sentence is true. It is also not the whole of what those two clauses require, and the half it
left out was **implemented backwards**.

## 1. What §8.7.4.5.7 states about an overlap, in two sentences the rows had not read

A patch is a *control* surface, and the clause says so in a NOTE of its own: "[t]he outline of a
projected square (that is, the painted area) need not be the same as the patch boundary if, for
example, the patch folds over on itself". A fold means one device point has more than one
preimage, and the clause decides which one wins — twice, at two scales:

> If more than one point ( u, v ) in parameter space is mapped to the same point in device space,
> the point selected shall be the one with the largest value of v . If multiple points have the
> same v , the one with the largest value of u shall be selected. If one patch overlaps another,
> the patch that appears later in the data stream shall paint over the earlier one.

§8.7.4.5.8 inherits both, being "identical to Type 6, except that they are based on a bicubic
tensor-product patch defined by 16 control points"; and in this tree one function tessellates both,
because a Coons patch *is* a tensor patch whose four interior points its boundary implies.

Neither ledger row mentioned either sentence. Neither row named a test, either — both said
`test = ["crates/pdf-model/tests/shadings.rs"]`, the whole file, which held not one type 6 or type
7 fixture. Those two facts are the same fact.

## 2. Where the precedence lives, which is not where one would look for it

Nothing in `pdf-model` or in any backend sorts a mesh's triangles or asks a depth question. Every
rasteriser here paints a mesh the same way: `MeshRaster::build` walks `triangles` in order and
`Triangle::paint` writes each covered pixel unconditionally, so **later in the vector is what the
reader sees**. The emission order in `mesh::tessellate` is therefore the whole of how that `shall`
is obeyed — a piece of load-bearing behaviour that looked like a loop.

The second sentence held: `patches` appends each patch's triangles in the order the stream states
its patches, so a later patch paints over an earlier one.

The first was inverted. `tessellate` nested the `u` step outside the `v` step, so among the cells
covering one point the last written was the one with the largest **u**, with `v` deciding only
between cells that share a `u`. That is the clause's rule and its tie-breaker exchanged: the
precedence is lexicographic in `(v, u)` and this tree ranked `(u, v)`.

The fix is the nesting. It is one edit and no arithmetic changes.

## 3. The fold that tells the rule from its tie-breaker

Most folds cannot see the difference, which is why this is a defect a corpus would not have
found and an eye would not have caught. The first three fixtures tried were reflex quadrilaterals
— a Coons patch with straight sides, whose surface is the bilinear map of its four corners. Every
one of them produced **zero** differing pixels between the two orders, and the reason is
structural: the two preimages of a point in such a fold lie on opposite sides of the fold line
along its normal, so the branch with the larger `v` is also the branch with the larger `u` and both
readings agree. A witness has to make the two disagree.

The construction that does is a patch folded along its own **diagonal**. Make the control points
symmetric — `p(i,j) = p(j,i)` for all sixteen, which for a Coons patch the derived interior points
inherit, and the derivation is checked: substituting the symmetry into the standard's own
expressions for `p12` and `p21` turns each into the other. Then

    S(u, v) = Σ Σ p(i,j) B(i,u) B(j,v) = S(v, u)

so every point off the diagonal is covered exactly twice, by `(u, v)` and by `(v, u)`. Give the
patch corner colours that depend on `v` alone — red at `v = 0`, blue at `v = 1` — and the clause's
answer at such a point is the colour of `max(u, v)` while the answer this tree gave was the colour
of `min(u, v)`. They differ everywhere the fold covers.

The fixture is two straight segments, (10,10)→(90,10) and (90,10)→(10,90), each serving as two of
the patch's four sides. At the pixel the test samples the two branches are 0.9 and 0.05 of the way
from red to blue. Against the pre-fix tree the test reports `246,0,9` where the clause asks for
blue — calibrated by putting the old nesting back and watching it fail, which is trap 13's rule.

`crates/pdf-model/tests/shadings.rs::a_folded_patchs_overlap_is_resolved_by_the_larger_parameter_v`,
and both rows now name it instead of naming a file.

## 4. What else was checked while the clause was open, and what was not

- **Table 84's and Table 85's continuations, index by index.** All three edge flags of both tables,
  for the points and for the two inherited colours, agree with `shared_edge` and with the stream
  order `control_grid` unpacks. So does the Coons interior derivation, term by term against the
  standard's four expressions.
- **A patch's byte alignment is a *choice*, and now says so.** §8.7.4.5.5's "[e]ach set of vertex
  data shall occupy a whole number of bytes" is about a vertex, and §8.7.4.5.7 states that a patch
  is laid out differently for precisely that reason. A patch has no vertices, so the sentence the
  cross-reference reaches has no referent, and the clause states nothing else about alignment.
  This tree does not pad, and that is recorded beside the code as a reading rather than left as a
  bare assertion. Nothing on this disk decides it: across `doc/pdf.js/test/pdfs` and the four
  `doc/corpora/` submodules — 1249 files — there are nine distinct type 6 and type 7 shadings and
  every one states `/BitsPerFlag 8` over whole-byte coordinate and component widths, so each patch
  is a whole number of bytes either way. A file with `/BitsPerFlag 2` is the witness that would
  decide it, and this tree has none.
- **`PATCH_STEPS` is still fixed**, so both rows stay `partial` for the reason they already gave.
  Nothing here touches it.

## 5. The general shape

A clause that states a rule and a tie-breaker in one sentence is a clause with an *order* in it,
and an order is the easiest thing to implement backwards, because both readings agree on every
input that does not exercise it. The tell here was not a picture — no corpus page in this tree
folds a patch diagonally — but the row: a `partial` note that names one debt and cites no test is a
note that has read one requirement of its clause. `--bin owed` ranked these two rows by the age of
their sentences, and the sentences were not wrong. They were short.
