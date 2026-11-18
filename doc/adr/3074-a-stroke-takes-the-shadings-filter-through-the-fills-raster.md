# 3074 — A stroke takes a shading's `/AntiAlias` filter through the fill's raster, on every backend

Session 2093. Status: **implemented.** Context: ADR 3024 (the filter, and the stroke it left unfiltered and
reported as `Unsupported::ShadingAntiAlias`), review 2038 finding D15. Clauses: ISO 32000-2 §8.7.4.3 Table 77,
§8.7.4.1. Supersedes ADR 3024's paragraph on the stroke.

## The reading

Table 77: "AntiAlias boolean ( Optional ) A flag indicating whether to filter the shading function to prevent
aliasing artifacts." The sentence names the shading's function and no painting operator, and §8.7.4.1 states the
operators a shading pattern paints with in one list: "a PDF content stream may use it with painting operators
such as f (fill), S (stroke), Tj (show text), or Do (paint external object) with an image mask". A stroke is
therefore owed the filter exactly as a fill is. ADR 3024 drew the stroke unfiltered only because its filter lived
in the fill's raster; that is a construction, not a reading, and it was reported as a shortfall.

## The construction

**The stroke takes the same raster, over its own ink.** `pdf_render::ShadingRaster::build` is still the one
place the filter is computed (trap 2); each backend now hands it the device pixels the stroke can reach instead of
a filled shape's, and draws the raster *through* the stroke instead of through the path:

- **`render-cpu`**: `shading::raster_shader` builds the raster over the stroke's `Extent` (the ink bounds and
  reach `ramp_shader` already uses, ADR 2404) and returns it as a nearest-neighbour pattern placed exactly as
  `ramp_shader`'s cells are placed. `CpuRasterizer::mark_brush` asks it first for a flagged shading, so every
  construction the stroke takes — the outline, the sub-pixel rule, the long mitres, the dots — paints the
  filtered raster with its own coverage. A raster that reaches no pixel of the surface is a transparent brush.
- **`render-raster`** (quorra): `stroke::raster_paint` uploads `Encoder::background_raster` over
  `stroke::ink_pixels` — the same bound, in device pixels — and strokes with `raster_scene::Paint::Mesh`, the
  paint a stroked mesh shading already used.
- **`render-gpu`** (Vello): `encode_stroke` draws `shading::fill_with_background` inside a layer clipped to the
  stroke's outline and its dots, as `encode_fill` draws it inside the filled shape. The union is one layer, so
  §11.6.2 composites the object once; dots carrying part of their coverage in their alpha are a second layer at
  that alpha.

`pdf_model` keeps the flag on a stroking pattern's shading and no longer reports it, so
`Unsupported::ShadingAntiAlias` is removed: no mark drops the flag any more (a glyph's stroke is a `Stroke`
command, and an image mask painted with a pattern is a fill). Table 77's `/Background` on a stroke is not part of
this decision; it is still dropped and reported (`Unsupported::ShadingBackground`), though the door built here is
the one it would take.

## Price

Only a stroke painting a flagged shading pays; every other stroke takes the code it took before. Measured as ADR
3024 measured the fill: one 612 × 792 page whose four quarters are each stroked twice, 30 units wide — the
quarter's rectangle inset by 20 (`re S`) and its diagonal (`m l S`) — with the same four shadings as ADR 3024's
arms (an axial and a radial over a four-step type 3 stitching, a type 1 step, the axial again), drawn once at
scale 1 by `callgrind_rasterise` in the dev profile: **0.638 G instructions without `/AntiAlias`, 11.52 G with
it** (×18.1). Wall clock on the loaded machine, five draws a run, the open and interpretation included once:
**62 ms a draw without, 156 to 165 ms with** (×2.6).

The ratio is above the fill's ×16.4 for a reason worth stating: the raster is evaluated over the stroke's
bounding box, not its outline (`ink_extent`, `ink_pixels`, `device_bounds`), so the inset rectangle pays for the
whole interior it does not paint, and each of the two strokes in a quarter evaluates its own raster. Confining
the evaluation to the outline's rows and spans would cut that, at the cost of a second region type in
`ShadingRaster::build`; it is not taken until a document shows the cost.

## Tests

`render-cpu/tests/all/anti_aliased_shading.rs::a_stroke_whose_shading_asks_for_anti_aliasing_is_filtered_at_its_step`
and `…::a_filtered_stroke_paints_the_fills_pixels_inside_it`;
`render-raster/tests/headless_quorra.rs::quorra_filters_a_stroke_whose_shading_asks_for_anti_aliasing`;
`render-gpu/tests/headless_gpu.rs::the_gpu_filters_a_stroke_whose_shading_asks_for_anti_aliasing`;
`pdf-model/tests/shadings.rs::a_stroking_patterns_anti_alias_is_filtered_as_a_fills_is`. The backends' tests draw
`test_scenes::shading_anti_alias_stroked`, the fill scene's three shadings painted by twenty-unit strokes. Each
backend's test fails with its stroke door shut, and the fixture with `pdf_model` dropping the flag.
