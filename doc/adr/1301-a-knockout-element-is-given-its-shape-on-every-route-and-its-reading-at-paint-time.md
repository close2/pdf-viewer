# 1301 — A knockout element is given its shape on every route, and its reading where it is painted

Status: accepted and **built**. Session 1232.
Amends: ADR 1265 (the refusal of §11.7.4.4's group inside an isolated knockout group) and the
statement-time record of §11.6.4.3's readings that ADR 0327 scoped.
Depends on: ADR 0151, ADR 0234, ADR 1022, ADR 1218, ADR 1256, ADR 1279.
Context: `crates/pdf-model/src/content/transparency.rs`, `crates/pdf-model/src/content/image.rs`,
`crates/pdf-model/src/image.rs`, `crates/pdf-model/src/content/path.rs`,
`crates/pdf-model/src/content/run.rs`, `crates/pdf-model/src/content/ext_gstate.rs`.
Clauses: ISO 32000-2 §11.3.6, §11.3.7.2, §11.3.7.3, §11.4.4, §11.4.6, §11.6.4.2, §11.6.4.3,
§11.6.4.4, §11.7.4.4.

`§N` is ISO 32000-2 and nothing else.

## 0. What the brief said, and what the tree said

The brief held that ADR 1279 had closed the residue six rows named — the product of shape and
opacity reaching a knockout element — and asked for each row to be re-derived. For an *image* that
held: `ImageSource::shape` answers on every image route. It did not hold for the one route where a
stencil is not an image command: §8.9.6.2's stencil painted through a pattern, which ADR 0151 turns
into a soft mask on the fill. And reading the rows' other remainders against the tree found two
more positions drawn wrong in silence and one refused that the clause answers.

## 1. A stencil painted through a pattern keeps its shape

- **Through a tiling pattern** the cells reach the page as a group carrying the stencil as its mask
  (`Interpreter::tile`), and `shape_without_the_mask_and_the_constants`'s group arm took every mask
  off. §11.6.4.2: "For image masks (8.9.6.2, "Stencil masking"), the shape shall be 1.0 for painted
  areas and 0.0 for masked areas" — so that mask is the group's shape, and the arm now keeps a
  stencil's mask as the fill arm always did. Planted back, the unpainted half is knocked out to the
  page.
- **Under an `/SMask` of its own** the fill's one mask was the stencil times §11.6.5.2's opacity,
  recorded as shape. The page is owed the product and the knockout is owed the stencil alone, so
  both are built: `image::decode_stencil_reporting_frame` returns the stencil beside the flattened
  raster, and `ShapeMasks::record_apart` pairs the drawn mask with a second mask holding it.
  §11.4.6's NOTE 5 gives the fixture's pixel: shape 1.0 at opacity 0 is "the colour and opacity that
  result from compositing the object with the initial backdrop", so the blue under it is knocked
  out; planted back, the blue survives.

## 2. §11.7.4.4's group inside an isolated knockout group

ADR 1265 refused the own-backdrop construction there, correctly, and the pair fell flat with a
report. NOTE 6 answers the position outright: "the initial backdrop of the inner group is the same
as that of the outer group", and an isolated knockout group's is transparent, where §11.3.6 says
"[a]n alpha value of αs = 0.0 or αb = 0.0 results in no blend mode effect". So the portions are
drawn Normal on transparency, any nested group isolated (`on_a_transparent_backdrop`) — exact
whatever they blend with. Three unit tests had passed `Some(KnockoutKind::Isolated)` while drawing
the answer straight onto the page; they now pass `None`, the context they draw in.

## 3. The reading a knockout element is built under

§11.6.4.4: the entry "shall determine whether the alpha constants are interpreted as shape values (
true ) or opacity values ( false )", and it is a graphics state parameter. The record of readings
painted under was kept at statement time, and two things made it wrong in silence:

- **`Q` did not restore it.** A group opening with `q /GA gs Q` kept the undone `true` and read every
  later constant as shape. `Q` now reports a restored reading like a `gs` does.
- **A record of both readings was replaced** when a reading was stated with nothing painted since the
  last, forgetting a reading something *had* been painted under. Only a record of one reading is
  replaced now: a reading kept too long is a report, a reading forgotten is a wrong shape.

And a `B`'s portions are read where they are painted — the operator's own state plus whatever a
tiling cell among them ran under (`open_parts_reading`, `close_parts_reading`) — rather than the
enclosing content's history, which refused a pair for a reading an earlier, finished object used.

## Consequences

- §11.3.7.2, §11.3.7.3 and §11.4.4 `partial` → `implemented`; §11.3.7 follows. §11.4.6 and
  §11.7.4.4 stay `partial`, narrowed: a non-isolated knockout group under a mode at its own `Do`
  whose elements blend under modes no construction moves (`render-cpu`'s `knockout_on_backdrop` has
  no result step), and content painted under both readings with a mask or constant — a tiling cell
  of the other reading, and a text object's portions, which still read the enclosing record.
- A shape channel every command carries is not owed by any of the six: §11.3.7.3's NOTE 2 licenses
  the product "whenever the independent shape and opacity are not needed", and the one reader that
  needs them is given a stated shape.
- Seven fixtures, each derived from the clause and each failing with its rule planted away.
  `raster_golden` holds 974 of 974: no tracked first page states any of these positions.
