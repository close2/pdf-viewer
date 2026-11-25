# Round 2250 — the window draws a page's own passes as `rasterize` does

A fix round in the `audit` worktree, taking audit 2248's fix group 1: B01 (the window's present
drew a page without its §10.5 transfer functions) and B02 (without §10.8.3's black list and spot
planes), and the §11.4.7 four-component page group's black list that 2248 read and did not run.

## The divergence

`QuorraRasterizer::rasterize` draws into a read-back and runs the passes after compositing (the
four-component pair, the curve, the cube, spot planes, crop, medium, transfer functions). The
window's `FrameSlot::scene_for` translated the commands into a scene drawn into the window texture
and read none of the side channels, and nothing refused such a page. The curve and the cube (a
`CalGray`/`CalRGB` page group) were dropped the same way, unreported.

## The fix (ADR 3701)

`render_raster::finish` holds the sequence once (`finish`, `draw_once`, `draw` with tiles).
`rasterize` calls it, and so does the window: a page that `needs_finishing` is drawn on frames of
its own through the same device and caches, finished onto 𝑊 inside the page and transparency
outside it, and put into the scene as an image in its place. Such a frame is never in page space,
and a still frame replays it. `tiles::render_in_tiles` takes a draw closure. Nothing is refused.

## Ledger

Rows before and after: 10.5 `implemented`, 10.6.1 `inapplicable`, 10.8.3, 11.4.7 and 11.7.5.2
`implemented`, unchanged. 10.8.3, 11.4.7 and 11.7.5.2 gain `finish.rs` and the window tests;
10.5 waits on B03 and B04.

**Gates.** Over `-p render-raster -p render-gpu -p render-cpu -p pdf-model -p viewer-device
-p viewer-qt -p viewer-ui`: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox` 0. nextest
`--lib --bins --tests` (`xvfb-run -a`, GPU live): 3864 run, 3 failed, which are the manifest's
2248-B03, B04 and B05; the two B01/B02 entries are removed. rustdoc of `render-raster` 0.
conformance 0. Turn: `personwithdog.pdf` has a `DeviceCMYK` page group, so the
window now draws two device frames for it, 31/29/38 ms on main against 106/106/150. Finishing the
page's own rectangle and a parallel §11.4.7 conversion bring it to 70/68/103. Its bands, and the Type 3
page's at 12 224 commands, are re-taken over two judged runs (A1071, A1073; ADR 3702).

**Habit.** A side channel on the display list has as many consumers as there are routes to a
picture; a grep for the channel's accessor across the backends is the census, and it found two
channels (`grey_curve`, `colour_cube`) that the audit had not named.
