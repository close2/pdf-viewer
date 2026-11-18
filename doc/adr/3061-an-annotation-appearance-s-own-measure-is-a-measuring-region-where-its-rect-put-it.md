# 3061 — An annotation appearance's own `/Measure` is a measuring region where §12.5.5 put it

Session 2090. Status: **implemented.** Context: ADR 3030's Cost. Clauses: ISO 32000-2 §12.5.5, §8.10.2 Table 95,
§12.10.2 Table 269.

ADR 3030 reported an image's or a form's own `/Measure` where its `Do` drew it, and left out "a form drawn as an
annotation's appearance", which is not a `Do` on the page. §12.5.5 settles whether it is a form: "Each appearance
stream is a form XObject (see 8.10, "Form XObjects")". Table 95's `/Measure` is a form's, and specifies "the scale
and units which shall apply to the form"; Table 269's `/LPTS` maps the unit square "to the rectangular bounds of the
Viewport, image XObject, or forms XObject that contains the measure dictionary". Nothing in either clause asks how
the form came to be drawn.

## Decision

1. **A stored appearance stream that states `/Measure` is a measured form**, reported in
   `Interpretation::measured_xobjects` beside the page's `Do`s, through the one `note_measured` ADR 3030 wrote.
2. **Its bounds are its `/BBox` carried by §12.5.5's `AA`** — `/Matrix` concatenated with the matrix `A` that fits
   the transformed box to `/Rect` — and the page's transform, which is the map the appearance is drawn with. So the
   region is where the annotation shows it, including §12.5.3's adjustment for a `/NoZoom` or `/NoRotate`
   annotation where one applies.
3. **A §12.7.4.3 construction states none.** It has no form dictionary of the file's, so there is no `/Measure` to
   read; a regenerated stored stream keeps its dictionary and is read like any stored one.
4. Ordering is ADR 3030's: annotations are drawn after the page's content, so an appearance is met first in
   `Viewports::at`'s reverse search, which is what drawing on top means.

## Cost

One `get_key` per stored appearance drawn. An annotation the interpretation did not draw (hidden by its flags or by
optional content) states no region, as an undrawn `Do` does not.

Test: `viewer-core` `annotation_appearance_measurement` (a quarter-turned `/Matrix` and an `A` that doubles; the
call removed, it fails).
