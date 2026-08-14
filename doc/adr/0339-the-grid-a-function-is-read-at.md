# 0339 — The grid a function is read at

**Status.** Accepted.

## Context

ISO 32000-2 §8.7.4.5.2, of a type 1 shading:

> In Type 1 (function-based) shadings, the colour at every point in the domain is defined by a
> specified mathematical function. The function need not be smooth or continuous.

*Every point* has no resolution, and this tree gave it one at the wrong moment: `pdf-model`'s
`function_based` evaluated the function over a fixed 128×128 grid (`FUNCTION_GRID`) while
building the display list, and `ShadingKind::Sampled` carried those pixels. Its own comment
called it "the one place in the display list where resolution is baked in". That is exactly the
interpret-time resolution decision ADR 0210 removed for a soft mask and ADR 0321 removed for a
JPEG 2000 codestream: the interpreter deliberately does not know the device scale —
`zooming_rasterises_again_without_interpreting_again` is what makes a display list
re-rasterisable at any zoom — so any grid chosen there is chosen for a magnification nobody has
asked for yet. `doc/todo/24` carried it as the vocabulary's last unclosed claimant.

A note on the clause number: the todo file (and the session's own instructions) said
§8.7.4.5.3. The standard's §8.7.4.5.2 is "Type 1 (function-based) shadings" and §8.7.4.5.3 is
"Type 2 (axial)" — principle 5 says the number comes from the document, so the code and the
ledger cite §8.7.4.5.2.

## Decision

### The display list names the colours and the device resolves them

`ShadingKind::Sampled` now carries the domain rectangle and a `DeferredColours` — an
`Arc<dyn ColoursAtDeviceScale>`, ADR 0210's shape moved onto `Paint::Shading`:

- **The contract is the deferred image's, restated for a paint.** `colours(Grid) ->
  ColourGrid` answers row-major colours on a grid **no finer than** asked. It is infallible for
  the same reason `samples` is: everything checkable without evaluating was checked and
  reported by the interpreter.
- **The grid is derived once, in `pdf-render`.** `Shading::sampled_at(page_to_device)` composes
  unit square → domain rectangle → shading matrix → device and asks `Grid::for_placement` — the
  same function that grids a deferred image — so the backends cannot ask the producer for
  different resolutions. That is trap 2's rule about device decisions, and it is what makes the
  cross-backend comparison on this paint evidence about the *drawing* rather than about two
  grids.
- **The producer lives in `pdf-model`.** `FunctionColours` holds the parsed `/Function` group,
  the resolved colour space, the compositing target (ADR 0220) and the domain — everything an
  evaluation needs and nothing borrowed from the document, because `Document` is not `Sync` and
  a display list is drawn on every core. Each cell is the function at the cell's *centre*,
  §10.7.4's half-pixel rule applied in the writing direction: at one cell per device pixel,
  each pixel carries the function's value at its own centre.
- **The bound is the producer's, and it is §10.7.3's to set.** A backend asks for the device
  pixels the domain covers, which a magnification the user chooses makes arbitrary, so
  `MAX_FUNCTION_CELLS = 2^22` halves both axes until the product fits — `MAX_MASK_GRID`'s
  construction, priced at 64 MB of transient `Color` and four million function evaluations at
  the limit. "[E]ach output device may have internal limits" is the clause's own licence for a
  device bound on shading resolution.
- **Opacity is answered without evaluating.** `ColoursAtDeviceScale::is_opaque` exists because
  the deferred *image*'s pessimistic constant is a fact about masks and would be a falsehood
  here: a conversion's alpha is the space's rather than the value's — §8.6.6.4's `/None`
  colourant, the one space whose colours are not opaque, discards its output for every tint —
  so one evaluation at the domain's corner answers §11.4.6's knockout question for the whole
  domain. §11.6.4.4's constant alpha wraps the producer (`DeferredColours::faded`) and scales
  each colour as it is produced.

### What each backend does with it

- **`render-cpu`** stretches the resolved grid over the domain as a `tiny-skia` pattern, as
  before — but the grid is now the device's, so the pattern's texel centres coincide with the
  pixel centres and the bilinear filter reads each texel exactly.
- **`render-quorra`** uploads the resolved grid as a transient image clipped to the filled
  path, as before, at the same grid by construction.
- **The Vello backend keeps its named refusal** (`brush_for`'s `UnsupportedPaint`). A grid is
  still not a brush any Vello gradient expresses; an image drawn inside a layer clipped to the
  shape — the mesh's and the cone's route — would genuinely fit and was deliberately not taken
  this round: the shipping presentation path is quorra, and a third construction is owed its
  own cross-backend evidence before it exists. `the_gpu_refuses_a_sampled_shading_by_name`
  pins that the refusal stays loud and that the CPU oracle draws what the device refuses.

### What guards it

Trap 2 binds a construction that differs between backends, and each guard was run with the fix
removed (a fixed 128-cell grid reintroduced in `sampled_at`) and watched fail:

- `render-cpu/tests/sampled_shading.rs` pins pixels against
  `test_scenes::sampled_colour_at`'s closed form at 1× and 4× — twelve waves sized so the old
  grid interpolates eleven levels wrong — and
  `zooming_resolves_the_shading_again_without_rebuilding_the_list` is the shading analogue of
  the standing zoom assertion: one list, two scales, a recording producer asked at 80×80 and
  then at 320×320.
- `pdf-model/tests/shadings.rs::a_function_based_shadings_discontinuity_lands_at_the_device_pixel`
  walks the whole route — file, interpreter, display list, backend — with the discontinuity the
  clause licenses: a step at domain 0.5 drawn across 400 device pixels lands red in column 199
  and blue in column 200, where the old grid mixed both.
- `test_scenes::sampled_shading` ends in a diagonal edge, and
  `cpu_and_quorra_agree_on_a_sampled_shading` compares the two constructions at 1× and 4× —
  the magnitude and the fractional coverage where they could part.

## Consequences

- The witnesses (`function_based_shading.pdf`, `function_based_shading_cmyk.pdf`) draw at
  device resolution; the visible change is the checkerboard swatch's edges, which the fixed
  grid blurred over ~3 device pixels at 4× and are now pixel-sharp. Gate movements are in
  `doc/history/504-*.md`.
- `doc/todo/24` closes and is deleted: this was its last open section. The fourth claimant —
  §8.9.6.3's explicit mask onto `ImageSource` — lives where it always did, in that clause's own
  ledger row, which now names both shapes of the vocabulary. The one residue (a soft mask
  *behind an image codec* does not decode at a chosen grid) stays named in
  `eligible_for_the_device_scale` and ADR 0321.
- A `DeferredColours` compares by identity, like `DeferredImage`: two `with_alpha` copies of
  one shading are equal only if they share the wrapper. Nothing in the tree compared a
  shading's pixels by value.
- The cost moved from interpret time to draw time, where it is paid per draw and bounded by
  `MAX_FUNCTION_CELLS`. On the corpus's witnesses the domains cover a few hundred pixels and
  the gates did not move measurably; the worst case (a full-page type 1 at deep zoom) is four
  million evaluations against the 16 384 the old grid always paid — the honest price of
  resolving at the device, and the same trade ADR 0210 recorded for masks.
