# 1568 — The first frame's span outside `FrameCost`'s stages is the offscreen lane's pass over its pixels

Status: accepted. Session 1366. Answers ADR 1558 section 1's last sentence, which left 0.5 to 3.1 ms
of the launch gate's first-frame span outside the stages `render_raster::FrameCost` names.
Supersedes nothing.
Context: ISO 32000-2 §11.4.7; ADRs 0387, 1531, 1558; `doc/habits/measuring.md` 66.
Code: none. The figures below were read by a scratch probe in an exported tree, since removed.

## 1. What is between the stages

The gate's span runs from the moment the viewer's request reaches the host to the moment
`draw_one_timed` has the pixels and has answered the request. `FrameCost::scene` and
`FrameCost::device` cover `render_whole`; what the span holds besides them is
`QuorraRasterizer::rasterize`'s own work after the device has handed the pixels back, and the
viewer's `RenderReady`.

A probe in an export of HEAD timed each part of `rasterize` on every child of one run of the gate
with `PDFVIEWER_LAUNCH_CLOCKS=1`, behind the heavy-walk lock (load 1.2 to 1.5):

| part | figure |
|---|---|
| `render_whole` from its first line to its pixels, of which `into_raster` | 6.4 to 7.5 ms, 0.005 to 0.07 ms |
| `settle` | 0.003 to 0.006 ms |
| the passes after it over the read-back pixels: premultiply, §11.4.7's medium imposed by `pdf_render::impose_within`, demultiply | **0.56 to 3.44 ms**: mean 1.55 at 707 × 1000, 1.92 at 708 × 1000, 2.18 at 773 × 1000 |

On the gate's five rows of the same run the span less `scene` and `device` was 0.7, 1.5, 2.2, 2.3 and
3.4 ms. So the span outside the stages is that pass, and the viewer's `RenderReady` is inside the
noise of it.

## 2. Decided: not a lever, and printed rather than taken

The pass is the offscreen lane's alone. A window's frame (`QuorraWindowRenderer::render`) and
`QuorraRasterizer::rasterize_frame` put the medium at the bottom of the scene, so the device
composites it and no pass over the pixels follows; a window pays none of the 0.6 to 3.4 ms, as it
pays none of the readback. It is in neither `raster-gpu`'s encode nor the presenter, which are what
a lever on the first frame would have to be in, and is not taken.

What it needs is a line of its own on the gate's print, beside the readback that stands in for a
present: `FrameCost::total` less `scene`, `device` and `settle` is the pass, and the gate can print
it with the stages it already reads (ADR 1558). `crates/viewer-ui/tests/launch_path.rs` is another
round's file this batch, so the print is handed over in this session's record rather than made.
