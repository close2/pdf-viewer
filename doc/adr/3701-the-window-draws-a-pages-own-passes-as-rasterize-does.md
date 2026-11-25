# 3701 — The window draws a page's own passes as `rasterize` does: one sequence, two callers

Session 2250. Status: accepted. Context: ISO 32000-2 §10.5, §10.8.3, §11.4.7, §11.7.5.2; review
2248 (B01, B02); ADRs 0351, 0702, 1317, 1329, 1472, 2150.

## The divergence

`QuorraRasterizer::rasterize` draws a page into a read-back and then runs the passes the clauses
put after compositing: §11.4.7's conversion out of the page group's blending space (the black list
of a four-component space drawn and multiplied back, a one-component curve, a three-component
cube), §10.8.3's spot planes, §14.11.2.1's crop, the medium, and §10.5's transfer functions chosen
per pixel by §11.7.5.2. The window's present (`present::FrameSlot::render`, which
`QuorraWindowRenderer::render` and `rasterize_frame` share and `viewer-device` draws every frame
through) translated the list's commands into a scene drawn straight into the window's texture. It
read none of `transfers()`, `black()`, `separation()`, `grey_curve()` or `colour_cube()`, and
nothing refused such a page: a page under `/TR` showed the file's colours before §10.5, and the
simulated press and a `DeviceCMYK` page group showed no black ink.

## Decision

The passes are one function, `render_raster::finish::finish`, and both routes call it.
`rasterize` calls it over its read-back; the window's `FrameSlot::scene_for`, before it builds a
scene, draws each page that `finish::needs_finishing` on frames of its own through the same device
and caches (`finish::draw_once`, which `rasterize`'s own frame is too) and finishes it with 𝑊
within the page and transparency beside it. The finished picture goes into the scene as an image in
the page's place, over the surround the scene already holds. Such a frame is never built in page
space (ADR 0702): the picture is of its view, so the scene's key is the placement and a still frame
replays it (ADR 0351) while a moved view builds it again. A target wider than the adapter takes
`tiles` as `rasterize` does, so there is no refusal to report.

The device composites; the processor runs the passes over the read-back, exactly the division
`rasterize` has. A shader for each pass is not this round's: the passes are `pdf_render`'s, and the
CPU oracle runs the same code, which is what keeps three routes from answering the clauses apart
(trap 2). A device form of a pass would be a fourth statement to keep equal.

## Cost

A page without a pass pays one predicate per page per frame (five `Option` reads); its scene,
encode and replay are unchanged, and the turn gate is the measurement.

A page with a pass pays, on a frame that builds its scene, one device frame per list (the page,
its black list, each spot plane, each run of §11.7.5.2's shapes) with a read-back, the passes and
one image upload, each over the page's own rectangle of the window rather than the window; on a
frame that keeps its scene, nothing more than any replay. The pages that pay are those with
`/TR` or `/TR2` in their marks, a reader's separation simulation, or a page group in a CIE-based or
four-component space. The last is not rare: Illustrator writes a `DeviceCMYK` page group, and
`personwithdog.pdf` on the turn gate is one, whose turn this moved. ADR 3702 has its figures, what
was cut and the band it proposes.

## Every route to the device

A display list reaches the device by two entry points, and both now run the passes:

- `QuorraWindowRenderer` (`FrameSlot::scene_for`), fixed here: `viewer-device`'s window frame
  (`renderer/device.rs`), its retained and as-read pages (`retained.rs`, `as_read.rs`), the sharp
  pass (`render_page_only`, `sharp.rs`), the idle `prepare`, and `quorra-confined`'s own frame
  (`viewer-ui/src/bin/quorra-confined/device.rs`). The idle `build_whole` replays a held scene. The
  processor's fallback frame is `render-cpu`'s picture uploaded through the same present, and it
  ran the passes already.
- `QuorraRasterizer::rasterize`, which ran them before: `viewer-device`'s `ReadBack` lane, which is
  how the Qt and GTK hosts draw.

The C-ABI session, printing in every host, presentation faces, video and the confined worker draw
on the processor; no route renders a thumbnail (a thumbnail shown is one the file holds).
`render-gpu` and the confined crossing refuse such a list by name, as they did.
