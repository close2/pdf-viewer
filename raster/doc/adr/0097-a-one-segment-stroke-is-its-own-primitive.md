# 0097 — A one-segment stroke is its own primitive, and the shader computes its area

Date: 2026-10-10. Status: **accepted, and built** — the caller's ADR 1817 §4 asked for it ("in
raster as a primitive of its own whose budget is counted per mark"), its ADR 1835 is the crossing.
Code: `raster-scene`'s `Command::Line` and `SceneBuilder::line`; `raster-gpu`'s `encode/line.rs`,
`encode/instance.rs` (`LINE_INSTANCE_STRIDE`), `shaders/line.wgsl`, the `Line*` pipeline kinds;
gate `tests/line_lane.rs`.

## The problem

The caller's witness is one sheet of 4 408 405 marks, 4 400 060 of them one-segment strokes a fifth
of a pixel long. Each was a `Command::Stroke` of its own uploaded outline: the encoder flattened,
expanded and scan-converted it on the host and drew it as a coverage quad, and every outline stayed
resident. That cost the frame 7 884 ms on `Cpu` and 8 409 ms on `Compute` (caller's ADR 1823), the
resource budget refused the generator's chop 16 (8 138 459 commands) for its one-segment outlines,
and the pre-count charged each mark 96 bytes.

## The decision

**A scene may state a stroke of one straight segment as `Command::Line`**: two points, the transform,
the width with §10.7.5's flag, the cap, a solid colour, a clip and a mask — no outline, no join (one
segment has none, ISO 32000-2 §8.4.3.4), no dash (the caller's), always `Normal`. Two equal points
are refused (`SceneError::InvalidLine`): §8.5.3.2's zero-length subpath is the caller's to settle.

**Under a transform that keeps the axes, inside a rectangular clip, the encoder writes one 60-byte
instance** — the device endpoints, the pen's two semi-axes, the cap, the colour, the clip's
rectangle — and `line.wgsl` computes the mark's exact area in each pixel. §8.4.3.2's set is the
points within half the width of the segment; the pen, the disc of the half-width, maps to an
ellipse on the device axes; dividing x and y by its semi-axes makes the pen the unit disc and keeps
the pixel an axis-aligned box. There the set is a band of half-width one, squared off (Table 53's
butt), carried one further (projecting), or finished by half discs (round). The band's area in the
box is a clipped quadrilateral's shoelace area; a half disc's is the unit disc inside the box cut by
the endpoint's square line, summed edge by edge as a triangle or a circular sector. The pieces share
no area, so their sum times the two semi-axes is the coverage — ADR 0005's exact-area rule, the
rectangle lane's, for a shape with round ends. The quad is the ink's parallelogram grown by √2/2 of a
pixel, so every pixel whose cell meets the ink is shaded and a long diagonal line pays for its own
band rather than its bounding box.

**Everything else takes the stroke road exactly**: a transform that turns the axes, a residue
clip, a segment the transform collapses. There the line is the two-point outline expanded and filled
as the stroke arm does, and draws a stroke's bytes (`tests/line_lane.rs` holds both to the bit).

**The pre-count charges a line 64 bytes** — the larger of its two roads, a line instance or a coverage
quad — and every other command 96 as before; the streams are reserved from the same split.

## Held by

`tests/line_lane.rs`, on every adapter here (RADV and llvmpipe) and both `Cpu` and `Compute`: 1 944
lines — widths 0.05 to 2 px, lengths 0.07 to 1.3 px, endpoints across pixel edges, all three caps,
four directions, unequal scales and a quarter turn — each pixel against the set's area as a convex
polygon clipped to the pixel in `f64`, round ends inscribed with 2 048 vertices: worst 0.594 levels
on RADV, 0.500 on llvmpipe; a dropped half disc or a butt drawn as projecting moves it to 232 and 152
levels. Against the stroke of the same outline under a similarity, butt and projecting ends within
one level, round within one (the stroker's chords). A rectangular clip met inside the pixel; the
residue and the turned transform drawing a stroke's bytes; the pre-count admitting 4 000 lines at
80 bytes a mark and refusing the same strokes at 96.

## Costs, written down

`sqrt`, `atan2` and `normalize` are implementation-precise, so a line is close, not identical,
across adapters (ADR 0082). One more module is parsed with the other nine on first need, and four
more pipelines compile lazily like every lane's — none is in the warm set. A frame of lines is
re-dispatched per command by the replay (`record_slow`), which is the stroke's own treatment.
