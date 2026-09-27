# 1340 — Shape is read apart from opacity only by a knockout group

Status: accepted. Session 1251. A reading, with one fixture; no construction changed.
Depends on: ADR 0234, ADR 1022 section 5, ADR 1205, ADR 1279, ADR 1301, ADR 1319.
Context: `crates/pdf-model/src/content/transparency.rs` (`stated_shape`,
`shape_without_the_mask_and_the_constants`), `crates/pdf-model/tests/transparency_groups.rs`.
Clauses: ISO 32000-2 §11.3.7.3, §11.4.3, §11.4.4, §11.4.6, §11.4.8, §11.5.2, §11.6.4.3.

`§N` is ISO 32000-2 and nothing else.

## The question

§11.4.3 asks that a group's result — "the resulting colour, shape, and opacity" — be treated as a
single object's. A raster holds their product. The row was `partial` on the chance that some
construction reads the pair where this tree carries the product.

## The readers, enumerated

- **§11.4.6** is the one the standard names: "The separate shape value shall be computed in any
  group that is subsequently used as an element of a knockout group."
- **§11.4.4 and §11.4.8.** §11.4.6: "When b = i - 1, the formulas simplify to the ones given in
  11.4.4". In §11.4.8's restatement (its formulas are not decoded in `doc/md/`; read in the printed
  standard) the source shape enters the group alpha only as the weight between the immediate and
  the initial backdrop, and with the backdrop index the immediate one those terms cancel. §11.4.4's
  NOTE 1: "Almost all of the computations use the product of shape and opacity (alpha) rather than
  opacity alone"; its NOTE 4: "There is never any need to compute the corresponding complete
  shape". The group shape `f` it returns is read only by a parent that is a knockout group.
- **§11.5.2** derives a mask from the group's alpha; Table 141's page composite reads only alpha.
- **§11.6.4.3's `/AIS`** moves a mask or constant between shape and opacity; the product is the
  same under both readings, so it too is visible only through a knockout weighting.

§11.3.7.3's NOTE 2 is then the licence for everything else: "This formula can be used whenever the
independent shape and opacity are not needed."

## Decision

Every route into a knockout group states the shape beside the product — `Command::Shaped` (ADR
0234), a group's union accumulated on transparency, isolated or not (ADR 1205), a stencil's own
samples (ADR 1279), each element under the `/AIS` it was painted under (ADR 1319) — so §11.4.3 is
`implemented`. A shape channel on every command (ADR 1022 section 5's price) would carry a quantity
nothing outside a knockout group reads, and is not owed.
`a_groups_opacity_does_not_become_its_shape_inside_a_knockout_group` holds the construction the
sentence names: a nested group whose opacity is its content's, isolated and not.
