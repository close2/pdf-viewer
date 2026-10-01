# 1472 — A target past the adapter's side limit is drawn in tiles and stitched

Session 1318. Status: accepted. Code: `crates/render-raster/src/tiles.rs`,
`QuorraRasterizer::render` in `crates/render-raster/src/lib.rs`.

## 1. The question

`issue19517.pdf` is 12 608 × 16 806 pixels at 1×; this adapter renders 16 384 pixels a side, so
the device refused the frame as a capability ("target 12608x16806 exceeds this adapter's limit"),
and `render-raster --test corpus` named it in `REFUSED_BY_THE_DEVICE`. Two readings were open:
record the refusal with the sentence that the viewer never asks for such a frame, or draw it.

## 2. What a reader sees, and what the oracle needs

The window never asks for this frame: it draws a viewport, and a viewport is at most the window,
so a reader at 100% sees the page through window-sized frames the device takes. But
`Rasterizer::rasterize` promises a raster of the whole target — the contract every backend
implements — and the correctness gate compares whole pages. A capability refusal there leaves one
page uncompared on the GPU lane for a reason no reader meets.

## 3. The decision

`QuorraRasterizer::render` draws a target wider or taller than `Limits::max_target_size` as square
tiles, each the same list under the target's transform followed by a whole-pixel translation, and
stitches the read-backs before every pass that runs over a readback (the four-component pair, the
curve and cube, the crop, the medium, the transfer functions), which therefore see one raster of the
whole target exactly as before. A whole-pixel translation moves no edge, sample or glyph phase
relative to the pixel grid. The tile side is the largest power of two within the side limit whose
RGBA target is at most a quarter of `max_frame_bytes` (4 096 here) — a choice, so that a tile leaves
the frame budget room for the page's own layers. Targets within the limit take the single-frame
path unchanged.

## 4. Measured

`issue19517.pdf` agrees with the oracle (mean error 0.0000), 2.5 s through raster against the CPU
backend's 1.2 s. Compared whole, the page costs memory: run alone it peaks at 7.9 GiB resident
(a raster of 847 MB per backend plus the comparison's own buffers), and the whole-corpus walk at
1× peaked at 10.7 GiB. That is inside the walk's 12 GiB bound, but not by much, and a later round
that sees the walk fail on memory should look at this page first.
