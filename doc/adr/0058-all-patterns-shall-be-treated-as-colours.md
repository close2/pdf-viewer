# ADR 0058 — "All patterns shall be treated as colours"

Status: accepted, 2026-07-31.

## Context

`pattern_text_embedded_font.pdf` was top of the ratio-ranked unexplained list after the previous
two sessions' pattern fixes, at 2.39 with a worst tile of 95.52 against a bound of 40.00. The
side-by-side says it in one look: the page draws two lines of `AbCdEf`, the second filled with a
shading and the first with a checkerboard **tiling** pattern. Three references draw both lines.
We drew the second and left the first blank.

## Why nothing was drawn

A tiling pattern is deliberately *not* a paint in this tree. `GraphicsState::fill_paint` says so
in a comment that has been right since the tenth session: "A tiling pattern is not a paint at
all — it is drawn by replaying its content stream — so it leaves the colour alone here."
`end_path` handles that by calling `tile` instead of pushing a `Fill`.

`show_text` did not. A glyph took `fill_paint()`, got the last solid colour set before the
pattern was selected, and painted with it — which on this page is nothing visible.

§8.7.2 closes the question in five words:

> All patterns shall be treated as colours; a Pattern colour space shall be established with the
> CS or cs operator just like other colour spaces

A glyph is filled with the fill colour. If the fill colour is a pattern, the glyph is filled
with the pattern. There is no sentence anywhere making text a special case, and §9.2.3's "[g]lyphs
may be painted … in any colour" is the other half of the same statement.

## Decision

`tile` takes the transform to tile under, instead of reading `state.transform`. `end_path` passes
`state.transform` as before; `fill_glyph` passes the **glyph's** transform, because a glyph
outline is in glyph space and the text rendering matrix is what places it.

`fill_glyph` is one function with the whole decision in it, called from the one place that fills
a glyph outline. That keeps `show_text` under the line limit and puts the clause beside the
choice it decides.

## What it cost, and that is the interesting half

**One page left the contradicted list and two documents left the comparison**, which is trap 5's
exchange running in both directions on one change:

- `scorecard_reduced.pdf` **strokes** with a tiling pattern. §8.7.2 makes a pattern a colour for
  `SCN` exactly as for `scn`, but the stroked *outline* is the backends' to compute (ADR 0028) —
  there is no path here to replay a cell across. It had been stroked in the last solid colour,
  silently; it is now named. That is a gap, not a permission, and §8.7.3's row says so.
- `ContentStreamCycleType3insideType3.pdf` reports `MAX_FORM_DEPTH`, and the document is named
  for the reason: its tiling pattern's `/Resources` name `/CyclicFont`, which **is** the Type 3
  font whose glyph the pattern fills. Tiling a glyph means entering that cycle; stopping at a
  bounded depth and saying so is what the bound is for. Before this change the cycle was never
  entered — because the pattern was ignored — and the page was quietly wrong.

`MAX_INCOMPLETE` therefore rises 95 → 97, which the ratchet allows only with the reasons written
down, and both reasons are on it. The oracle's judged set falls by two pages; 820 agree and 77
are contradicted, from 820 and 78.

## Consequences

- §8.7.3 is `partial` rather than `implemented`, and what is missing is named: the stroke half.
- `CONTRADICTED_UNEXPLAINED` is 31.
- Three sessions in a row have found a defect in the pattern machinery — the cell's `/BBox`
  (ADR 0056), the space a pattern inside a form maps to (ADR 0057), and text as a pattern's
  target. All three were invisible to the corpus gate, which reports what cannot be *built*, and
  visible to the oracle, which asks whether what was built is right.
