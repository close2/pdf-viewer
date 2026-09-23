# 1306 — A reading of `/AIS` is kept by the scope that paints under it

Status: accepted and **built**. Session 1234.
Amends: ADR 1301 (the record of §11.6.4.3's readings, which it scoped to a path's portions).
Depends on: ADR 0327, ADR 0415, ADR 1205, ADR 1265.
Context: `crates/pdf-model/src/content/{transparency,pattern,path,text,run}.rs`,
`crates/pdf-model/src/image.rs` (`ShapeMasks::record`).
Clauses: ISO 32000-2 §8.7.3.1, §9.3.8, §11.6.2, §11.6.4.2, §11.6.4.3, §11.6.4.4, §11.6.7,
§11.7.4.4.

`§N` is ISO 32000-2 and nothing else.

## What the clauses say

§11.6.4.4: the entry "shall determine whether the alpha constants are interpreted as shape values
( true ) or opacity values ( false )", and it is a graphics state parameter. A tiling cell runs
under "the graphics state that was in effect at the beginning of the pattern's parent content
stream" (§8.7.3.1), and §11.6.7 adds "[t]he definition shall not inherit the current values of the
graphics state parameters at the time it is evaluated", while making what the cell evaluates to
"the object's source colour ( 𝐶𝑠 ), object shape ( f j ), and object opacity ( qi )". So a cell's
marks take the cell's reading, which is a fact of the pattern and not of the mark painting it —
interpreting the cell once and copying it (ADR 0430) is right — and the mark's reading governs its
own constant and mask only.

## What the tree did, and does

- **A cell's reading was never recorded.** The cell starts from `GraphicsState::initial`, and a
  cell stating no `/AIS` added nothing to the record, so under a page's `/AIS true` its `ca ½` was
  read as shape: `(128, 127, 0)` for `(255, 128, 128)`, silently. `run_cell` now opens a record of
  its own (`open_reading_scope`, `close_reading_scope`) and `compose_tiling` decides what reaches
  the enclosing one.
- **A group or cell whose raster is its own shape keeps its readings to itself.** Where
  `alpha_is_shape` holds, the group's drawn alpha is its shape under the reading its content ran
  under, so no enclosing reading can reinterpret it: `run_transparency_group` and `compose_tiling`
  fold the readings in only where it does not, and `shape_without_the_mask_and_the_constants` takes
  such a group's elements as their own shapes rather than stripping constants a different reading
  made shape. Tiles that would stand inline under a reading other than the mark's are given
  §11.6.7's isolated group where its NOTE 1 makes that exact. `raster_golden` moved three first
  pages' lists and no pixel: `issue18032.pdf`, `knockout_inner_backdrop.pdf` and
  `knockout_nested_group_alpha.pdf` state the new shape of a group that already was its own.
- **A text object and each glyph pair have their own record**, opened at `BT` and at each combining
  glyph, so §9.3.8's group and §11.7.4.4's pair read what their glyphs were shown under rather than
  the enclosing content's history.
- **A part painted through a translucent cell composites**, which the state at the operator does
  not say: `tile` answers whether the cell's marks composite, and `B` and a mode 2 or 6 glyph make
  the pair one object on it (§11.6.2). Building it found the knockout shape of a tiled stroke drop
  the stroke's outline mask as opacity, knocking out the whole cell; `stroke_shape` records the
  mask as the shape §11.6.4.2 makes it.

## What is left, and why

Content that paints under both readings with a mask or a constant as *direct* elements of one
scope, and a nested group or cell under the other reading whose raster is not its own shape. Each
needs the reading an element was painted under carried beside that element; a pre-stated
`Command::Shaped` would carry it, but `render-raster` relies on that command occurring only inside
a knockout group, and a pair or group refused for another reason would carry it out. Both are
reported by name. The cell's starting reading is taken from `GraphicsState::initial`, which is the
page's; a pattern named in a form whose `Do` stated `/AIS true` would start from the form's, and
that is §8.7.3.1's inheritance of every parameter rather than this one's.
