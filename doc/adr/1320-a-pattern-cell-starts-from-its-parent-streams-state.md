# 1320 — A pattern cell starts from its parent stream's state, and a tiling is one knockout element

Status: accepted and **built**. Session 1241.
Amends: ADR 1306 (whose cell started from §8.4's initial state).
Depends on: ADR 0430, ADR 1266, ADR 1319.
Context: `crates/pdf-model/src/content/pattern.rs` (`PatternInitial::stream`,
`Interpreter::cell_state`, `wrap_inline_tiles`), `content/text.rs` (`knockout_by_cells`),
`crates/render-cpu/src/lib.rs` (`coverage_twin`).
Clauses: ISO 32000-2 §8.7.2, §8.7.3.1, §9.3.8, §11.4.4, §11.4.6, §11.6.7.

`§N` is ISO 32000-2 and nothing else.

## 1. The cell's starting state

§8.7.3.1's step b): "Installs the graphics state that was in effect at the beginning of the
pattern's parent content stream". For a form §8.7.2 names that state: "the form coordinate space at
the time the form is painted with the Do operator". Every nested run saves the state it begins
with in `PatternInitial::stream`, and `cell_state` installs it with §11.6.7's resets — "blend mode,
soft mask, and alpha constant" — and nothing else, so `/AIS`, colours, line width and text state
carry. The transfer function carries too and changes nothing: a cell's marks take §11.7.5.2's
function from the painting mark (ADR 1266).

`ContentStreamNoCycleType3insideType3.pdf` moves for it, by the clause: its cell of text sits in a
Type 3 glyph whose description began in rendering mode 2 with a red stroke 20 units wide, and it
strokes so now. Its producer's comment says magenta; the state it installed says otherwise.
`render-raster` leaves specks along that cell's seams and the page is on its differing list.

## 2. A glyph translucent only through its cell

§9.3.8's group is owed where glyphs composite and overlap; §11.6.7 makes a cell's result the
glyph's own "object opacity ( qi )", so `knockout_by_cells` counts what `tile` answers.

## 3. A tiling is one element

§11.4.4's NOTE 5 flattens a group only where it "has the same knockout attribute as its parent
group". Inside a text object, a combined fill and stroke or a knockout group, inline tiles whose
marks composite are given §11.6.7's non-knockout group, so the cell's marks composite with one
another and not knock one another out.

## 4. `render-cpu`'s bare knockout element under a clip

A bare knockout draw is Porter-Duff Source by coverage, and `tiny-skia` applies a clip mask to
Source by scaling the source: where the mask is below 1.0 inside the path's reach, the accumulation
was cleared. An element whose clip cuts it is drawn as `(1 − f) × P + S` — Destination-Out with an
opaque twin, then Plus. Seven tracked first pages move, by at most a few levels, at clip edges
(`22060_A1_01_Plans`, `issue12810`, `issue13447`, `issue17069`, `issue17215`, `issue1905`,
`knockout_groups_test`); `issue17215`'s anti-aliased edge row was the one cut off.
