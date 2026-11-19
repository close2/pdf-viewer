# 3094 — A stroke is washed with its shading's `/Background`, on every backend

Session 2098. Status: **implemented.** Context: ADR 0529 (the wash is one raster with the shading), ADR 3074 (a
stroke takes the fill's raster for `/AntiAlias`), which left the stroke's `/Background` dropped and reported as
`Unsupported::ShadingBackground`. Clauses: ISO 32000-2 §8.7.4.3 Table 77, §8.7.4.1, §8.7.4.2, §11.6.7.

## The reading

Table 77: "If present, this colour shall be used, before any painting operation involving the shading, to fill
those portions of the area to be painted that lie outside the bounds of the shading object." and "The background
colour shall be applied only when the shading is used as part of a shading pattern, not when painted directly
with the sh operator." The only exclusion the table states is `sh` (§8.7.4.2 says it a second time: "The
Background entry, if present, is ignored"). A stroke painted with a shading pattern is "a painting operation
involving the shading" made as part of a shading pattern — §8.7.4.1 lists "S (stroke)" among the operators a
shading pattern paints with — and its "area to be painted" is the stroke's outline. So the wash is owed to a
stroke over the part of its outline the shading does not reach, and to nothing beyond the outline.

## The construction

`pdf_model` no longer drops the entry for a stroking selection: `ShadingDefinition::paints_background` is
removed, and a shading pattern's shading keeps its `/Background` whichever operator selected it; `sh` still drops
it where it builds its own shading. All three backends already route a stroke whose shading
`is_rastered()` — `/Background` or `/AntiAlias` — through the raster a fill takes over the stroke's ink (ADR
3074): `render-cpu`'s `raster_shader`, quorra's `raster_paint`, Vello's `stroke_filtered`. That raster is
`pdf_render::ShadingRaster`, which is the wash and the shading as one painting operation (§11.6.7), so the
stroke paints exactly the fill's pixels inside its own coverage. `Interpreter::paint_clip` already leaves a
washed type 1 shading's domain clip off for a stroke as for a fill.

`Unsupported::ShadingBackground` stays, for the one case left: an array whose length is not the colour space's
component count, which states no colour.

## Tests

`test_scenes::shading_background_stroked` strokes `shading_background`'s four quadrants (axial, radial, mesh,
sampled) along their middles, twenty units wide. `render-cpu/tests/all/stroked_shading_background.rs` (the wash at
each stroke's end, nothing above it; the fill's very pixels inside the stroke);
`render-raster/tests/headless_quorra.rs::quorra_washes_a_stroke_whose_shading_states_a_background`;
`render-gpu/tests/headless_gpu.rs::the_gpu_washes_a_stroke_whose_shading_states_a_background`;
`pdf-model/tests/shadings.rs::a_stroking_patterns_background_is_painted_along_the_stroke` (replacing the test
that asserted the report). The backends' tests exercise doors ADR 3074 opened; what fails without this decision is
the `pdf-model` test, because the backends never received the colour.
