# 1234 — A knockout group under its own mode, and a reading kept where it is painted

Date: 2026-09-23. Branch `batch-1233-1238`, shared worktree. ADRs 1305, 1306.

## What the brief said, and what the tree said

The brief named three residues. The tree held them, and two silent wrong pixels beside them: a
tiling cell's `/AIS` reading was never recorded, and a tiled stroke's knockout shape dropped the
stroke's outline. A `B` whose translucency came only from a pattern cell was drawn as two marks.

## Built

- ADR 1305: `knockout_on_backdrop` keeps §11.4.8's group alpha beside its accumulation under a
  non-Normal `Do` and takes §11.4.4's result step. The refusal is gone. The fixture's
  `(255, 128, 96)` was worked by hand from §11.3.5.
- ADR 1306: a tiling cell, a text object and each glyph pair keep their own record of
  §11.6.4.3's readings. A group or cell whose raster is its own shape keeps its readings out of
  the enclosing record. A part painted through a translucent cell makes a `B` or a mode 2 glyph
  one object, and a tiled stroke's outline mask is recorded as a shape.

## Rows

None changed status. §11.4.6 and §11.7.4.4 stay `partial`, narrowed. What is left in both is an
element that needs its own reading carried beside it: content painted under both readings with a
mask or a constant within one scope, or a cell under the other reading whose raster is not its
shape. §11.4 stays `partial` because of §11.4.3's shape channel, that §11.4.6 remainder, and
§11.4.7's reference XObject.

## Gates

`raster_golden` moved three pages. The lists changed and no pixel did. They are regenerated.
