# 2083 — A shading's `/AntiAlias true` is honoured on every backend

Session 2083, a pixels round in `pdf-viewer-lane4`, owed by the owner's A899 ("agreed" to Q899's option (a)) for
review 2038's finding D15 (G033 of `doc/reviews/2038-specification-audit/fixes.md`).

- **Premise held.** Table 77: "AntiAlias boolean ( Optional ) A flag indicating whether to filter the shading
  function to prevent aliasing artifacts", no filter named, default false; nothing in `crates/` read it.
- **ADR 3024**: a box over the device pixel on a regular 4 × 4 sub-pixel lattice, premultiplied mean
  (`pdf_render::FILTER_SAMPLES`). It supersedes ADR 0037's listing of the entry as a hint like flatness.
- `pdf_render::Shading::anti_alias`, set by `pdf_colour::shading` where there is a function to filter (types
  1–3, a mesh with `/Function`). `Shading::is_rastered` sends a flagged fill through `ShadingRaster` on all three
  backends — the `/Background` door — which paints types 2, 3 and parametric meshes a band at a time on a lattice
  four times finer and averages it down. Type 1's producer (`FunctionColours`) averages the same sub-cell lattice
  and is built without a device program. `sh` keeps the flag; a stroking pattern drops it and is reported as
  the new `Unsupported::ShadingAntiAlias`. The flag crosses the confined boundary (`least::SHADING` + 1).
- Without the flag nothing moves: the unflagged path is the same code, and no test or golden changed.
- **Tests**: a stepped function through a pixel centre in `test_scenes::shading_anti_alias` (axial, radial,
  parametric mesh) and in PDF fixtures (a type 3 stitching of constant pieces; a type 1 step). A plant
  returning each backend's door to `background.is_some()` fails `render-cpu`'s, `render-raster`'s and
  `render-gpu`'s tests and the `pdf-model` fixture's.
- **Price** (one 612 × 792 page of four flagged full-quarter shadings, scale 1, dev profile): 0.458 G instructions
  without the flag, 7.53 G with it (×16.4); 46 ms against 112 ms a draw by wall clock (×2.4, the bands run in
  parallel). In ADR 3024.
- Ledger 8.7.4.3 stays `implemented` (its final state), the note says what is built and the tests are named.
  D15's line in `fixes.md` is `fixed (A899, ADR 3024)`. No owner audit file was edited.

**Gates.** Over one `-p` set (pdf-render, pdf-colour, pdf-model, render-cpu, render-raster, render-gpu,
viewer-confined, viewer-core, pdf-transform, test-scenes): fmt `--check` exit 0; clippy `-D warnings
--all-targets` exit 0 after three arithmetic fixes; `pdf-sandbox --bins` built; nextest `--lib --bins --tests`
5870 run, 3 failed — the manifest's 1 and this round's two `pdf-model` fixtures, which found the `sh` slip named below
and pass after it (pdf-model again: 2873 run, 0 failed); rustdoc `-D warnings --document-private-items` exit 0
after one `\[a]`; `fuzz` `cargo check --bins` exit 0; conformance exit 0 (after the ADR stopped naming a path outside the tree).

Trap (proposed): a mechanical insertion of a new field's default beside every `background: None` put
`anti_alias: false` into the one literal that meant to keep it — `sh`'s — and only the fixture caught it. Insert
by compiler error, never by pattern.
