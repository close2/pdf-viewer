# 3024 — A shading's `/AntiAlias true` is a box of sixteen sub-pixel samples, on every backend

Session 2083. Status: **implemented.** Context: review 2038 finding D15, `doc/questions/A899` (the owner's
"agreed" to Q899's option (a)). Clause: ISO 32000-2 §8.7.4.3, Table 77.

Table 77: "AntiAlias boolean ( Optional ) A flag indicating whether to filter the shading function to prevent
aliasing artifacts." Its NOTE 3 says why — the operators "sample shading functions at a rate determined by the
resolution of the output device" — and the default is false. The entry names no filter, so the filter is a
choice, and this is it. It supersedes ADR 0037's listing of the entry as "a hint … in the same class as §10.7.2's
flatness": §10.7.2 permits a processor to ignore flatness, and no sentence permits it to ignore this flag.

## The filter

**A box over the device pixel, sampled on a regular 4 × 4 lattice** (`pdf_render::FILTER_SAMPLES`): a pixel's
colour is the mean of the shading at the sixteen points `(x + (2i + 1)/8, y + (2j + 1)/8)`, each carried into
shading space by the inverse of the drawing transform — so the footprint is the pixel's own, whatever the
transform. The mean is of premultiplied colour, so a sample the shading leaves unpainted lowers coverage rather
than darkening colour. Why these two choices:

- **A box, not a tent or a Gaussian.** NOTE 3's aliasing is a step finer than a pixel; a box over the pixel's
  own area is the filter that answers "what fraction of this pixel is which side", reaches no neighbour, and
  leaves a pixel a whole pixel from the step untouched. A wider kernel blurs what the producer drew sharp on
  purpose and gives each backend a neighbourhood to disagree about.
- **Sixteen samples.** A step then lands in one of seventeen levels of its pixel, about one in fifteen of the
  8-bit channel — the stair the eye sees is gone and the cost stays bounded. Sixty-four would quadruple the
  price to resolve levels a screen barely shows.

## Where it is applied

**One function, so the three backends cannot differ** (trap 2). `pdf_render::Shading::anti_alias` carries the
flag; `Shading::is_rastered` sends such a fill through `pdf_render::ShadingRaster`, which every backend already
draws for `/Background` (ADR 0529). For types 2 and 3 and a parametric mesh the raster is painted a band at a
time on a lattice four times finer than the device and averaged down (`filtered`, `box_down`), so the colour at
each sample is exactly the unfiltered code's colour at that point. A **type 1** shading's colours are made
upstream on a lattice the device chose to be its pixels, so its producer (`pdf_colour::shading::FunctionColours`)
evaluates the same 4 × 4 sub-cell centres and averages them, and the shading is built without a device program,
which would sample each pixel once. A **mesh whose corners carry colours** has no function, so the flag asks
nothing of it and is not set (`pdf_colour::shading::filters`).

A **stroke** is drawn with the backend's own gradient, not through the fill's raster, so a stroking pattern's
shading drops the flag and the interpreter names it (`Unsupported::ShadingAntiAlias`), as it names the stroke's
unpainted `/Background`.

## Price

Up to sixteen evaluations a pixel, and only for a shading that asked. Measured on one 612 × 792 page of four
shadings each filling a quarter — an axial and a radial over a four-step type 3 stitching, a type 1 step, and the
axial again — drawn once at scale 1 by `callgrind_rasterise` (dev profile, `opt-level = 1`): **0.458 G
instructions without `/AntiAlias`, 7.53 G with it** (×16.4; the interpretation, identical in both, is inside both).
Wall clock on the loaded machine, five draws each: **46 ms a draw without, 112 ms with** (×2.4): the filtered
raster is painted in bands across the thread pool (`FILTER_ROWS`), where the unfiltered gradients run on one
thread. The two arms are one eight-object file that differs only in the entry: `/Coords [0 0 612 50]` for the axial,
`[459 198 0 459 198 190]` with `/Extend [false true]` for the radial, `{ pop 0.5 ge { 1 1 1 } { 0 0 0 } ifelse }`
over a `/Matrix [306 0 0 396 0 396]` for the type 1, each painted by `sh` inside a quarter-page clip. No golden moves: a shading without the
entry takes the same code it took before.

## Tests

`pdf-render::shading::tests::an_anti_aliased_axial_step_is_the_mean_of_its_sub_pixel_samples` and
`…_parametric_mesh_is_filtered_and_one_with_a_background_is_not`; `pdf-colour::shading::tests::anti_alias_asks_for_a_filter_only_where_there_is_a_function_to_filter`;
`pdf-model/tests/shadings.rs::an_anti_aliased_stitched_step_is_filtered_and_an_unflagged_one_is_not`,
`…::an_anti_aliased_function_based_step_is_filtered_by_its_producer`,
`…::a_stroking_patterns_anti_alias_is_reported_and_a_fills_is_not`; `render-cpu/tests/all/anti_aliased_shading.rs`;
`render-raster/tests/headless_quorra.rs::quorra_filters_a_shading_that_asks_for_anti_aliasing`;
`render-gpu/tests/headless_gpu.rs::the_gpu_filters_a_shading_that_asks_for_anti_aliasing`. Each backend's test
fails on a backend that draws a flagged shading its own way.
