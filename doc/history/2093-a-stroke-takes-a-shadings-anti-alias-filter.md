# 2093 — A stroke takes a shading's `/AntiAlias` filter, on every backend

Session 2093, a pixels round in `pdf-viewer-lane3`, the stroke half of review 2038's finding D15 that round 2083
left reported (`Unsupported::ShadingAntiAlias`).

- **Premise held.** Table 77's `/AntiAlias` asks "to filter the shading function" and names no operator;
  §8.7.4.1 (the brief said §8.7.3.1, which is the tiling patterns' clause) lists "S (stroke)" beside "f (fill)"
  among the operators a shading pattern paints with. Round 2083 left the stroke unfiltered only because the
  filter was in the fill's raster.
- **ADR 3074**: the stroke takes the same `pdf_render::ShadingRaster` over the pixels its ink can reach, and
  draws it through its own coverage. `render-cpu`: `shading::raster_shader`, asked first by `mark_brush`, a
  nearest-neighbour pattern placed as `ramp_shader`'s cells are, so every stroke construction (outline,
  sub-pixel rule, mitres, dots) paints it. `render-raster`: `raster_paint` strokes with `Paint::Mesh` of
  `background_raster` over `ink_pixels`. `render-gpu`: `stroke_filtered` draws `fill_with_background` in a layer
  clipped to the stroke's outline and dots (one layer for the union, §11.6.2).
- `pdf_model` keeps the flag on a stroking pattern; `Unsupported::ShadingAntiAlias` is removed (match arms in
  `viewer-core`, `pdf-transform`, the corpus test). No mark drops the flag now: a glyph's stroke is a `Stroke`
  command, an image mask with a pattern is a fill. A stroke's `/Background` is still dropped and reported.
- **Tests**: `test_scenes::shading_anti_alias_stroked` (the fill scene's axial, radial and parametric mesh
  painted by twenty-unit strokes); one test per backend, a CPU test that the stroke paints the fill's very
  pixels inside it, and a `pdf-model` fixture that the flag reaches the list and the stroke is filtered.
  Plant (each door shut, the model dropping the flag): all five fail; reverted.
- **Price**: in ADR 3074. Only a stroke painting a flagged shading takes new code, so a golden page moves only
  where one does; `raster_golden` is the train's to run, and was not run here.
- Ledger 8.7.4.3 stays `implemented` (its final state); the note's stroke sentence is rewritten and the five
  tests are named. No owner audit file was edited.

**Gates.** Over one `-p` set (pdf-colour, pdf-model, render-cpu, render-raster, render-gpu, test-scenes,
viewer-core, pdf-transform): fmt `--check` exit 0; clippy `-D warnings --all-targets` exit 0 after two
`too_many_lines` splits and one `too_many_arguments` pairing; `pdf-sandbox --bins` built; nextest `--lib --bins
--tests` under `xvfb-run -a`, unpinned (the RX 580): 5445 run, 0 failed, matching the manifest's 0; rustdoc `-D
warnings --document-private-items` exit 0; `fuzz` `cargo check --bins` exit 0; conformance exit 0.

Left: a stroke's Table 77 `/Background` (the same door would carry it); a stroke with an *unflagged* mesh shading
is refused by `render-cpu` (`UnsupportedPaint`) and Vello, while quorra draws it.
