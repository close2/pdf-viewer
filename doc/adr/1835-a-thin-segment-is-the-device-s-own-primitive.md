# 1835 — A thin segment is the device's own primitive

Session 1499. Status: **accepted** and **built**. Asked by the project owner on 2026-10-10 —
"create the biggest PDF which can be rendered on the GPU and continue improving the GPU path as
well" — as ADR 1817 §4's second increment (`doc/todo/69` §2), after ADR 1823. Context: ADRs 0082,
0085, 0701, 0702, 1817, 1823, 1829; raster's ADRs 0005, 0080, 0095, 0097. Code:
`crates/render-raster/src/stroke.rs` (`thin_line`); raster's `Command::Line`, `encode/line.rs`,
`shaders/line.wgsl`. Instrument: `crates/render-raster/examples/heavy_drawing.rs`. Clauses: §8.4.3.2,
§8.4.3.3, §10.7.4; no ledger row moves.

## 1. The premise, and the hypothesis that did not hold as stated

ADR 1823 left the device two walls on the owner's drawing: host encode (7 884 ms on `Cpu`, 8 409 ms on
`Compute`, against the oracle's 7 590 ms) and `max_resource_bytes` at chop 16, an outline resident for
every one-segment stroke. Both held in the code: `render-raster` uploaded each stroke's path as its
own outline, and the encoder flattened, expanded and scan-converted each on the host.

The brief's hypothesis — "the primitive's instance is 32 B and needs no quad, so the pre-count halves"
— did not hold as stated. A line instance is **60 B**: the two device endpoints, the pen's two
semi-axes and the cap, the colour, and the clip's rectangle, which the shader must meet per pixel
because a round end's boundary is not a rectangle's and cannot be clipped on the host. And a line the
closed form does not reach (a residue clip, a transform that turns the axes) writes a coverage quad,
so the pre-count charges a line **64 B**, the larger of its two roads, against 96 for every other
command — a third off, not half. The frame's host encode falls by far more than the pre-count does,
because what it lost is the flattening and expansion, not the bytes.

## 2. The decision

**`render-raster` asks `pdf_render::thin_segment` (ADR 1829, the one copy every backend asks — trap
2) and sends every stroke it admits as one `Command::Line`**: the path's two points, the placement,
the width already resolved through §8.4.3.2's zero and §10.7.5 in the path's own space, the cap, the
colour, the clip and the mask. No outline is uploaded for it. Raster's pen is half that width under
each of the transform's two stretches, which is `ThinSegment`'s pen, and its area in a pixel is the
area `ThinSegment::coverage` states: §8.4.3.2's set with Table 53's end, by §10.7.4's pixel.

**A scene holding one is true only at its view** (ADR 0702): the two-by-two block is this view's
pixels, so the scene consumes the view exactly as §8.5.3.2's dots do. Raster's line is exact at every
size, so a later argument may declare such a scene true at every view; that argument is not made
here, because it would leave the device and the processor drawing different marks by different
constructions after a zoom.

The primitive itself is raster's ADR 0097: the area in the frame where the pen is the unit disc — the
band a clipped polygon, a round end the disc inside the pixel cut by the endpoint's square line,
summed edge by edge as triangles and sectors — in any direction under a transform that keeps the axes.

## 3. What it moved

Measured headless on the real adapter (RADV, Radeon 890M) in one clock hold, HEAD's export and this
round's alternating arm by arm, each tree built in its own target directory; HEAD's processor is
the oracle in both, so the device's own change is what moves (`heavy_drawing`, the oracle at four
strips):

| page | lane | HEAD | after |
|---|---|---|---|
| the owner's drawing at the fit (1416 × 1000) | `Cpu` | 9 666 ms: scene 2 560, encode 6 910 | **1 204 ms**: scene 327, encode 672 |
| | `Compute` | 9 755 ms | **1 221 ms** |
| generator chop 8 (4 091 099 commands) | `Cpu` / `Compute` | 9 796 / 9 832 ms | 1 119 / 1 278 ms |
| chop 12 (6 114 779) | both | refused: 536 870 968 resource bytes | 1 653 / 1 701 ms |
| chop 16 (8 138 459) | both | refused the same way | 2 731 / 2 188 ms |
| chop 24 (12 185 819) | both | — | 2 902 / 4 026 ms |
| chop 1 (549 659), no mark admitted | both | 853 / 904 ms | 863 / 1 017 ms, **the same bytes** |

The drawing's frame now uploads 32 791 resources, where every stroke was one; the generator's
14 464 at every chop from 4 up. So **the biggest page the device draws** went from chop 8 to chop
24 — the largest size tried, 12 185 819 commands, which the interpreter's 2 GiB list (ADR 1810)
admits — and the resource budget is no longer met by one-segment strokes. The six corpus arms against HEAD's (`/home/AI/arms-1496/`) are in the record.

## 4. Agreement, and what is not claimed

Each line's pixels carry the set's area to 0.594 levels on RADV and 0.500 on llvmpipe, against the
set as an `f64` polygon (raster's `tests/line_lane.rs`, 1 944 lines). On the owner's drawing the
device and the processor — with ADR 1829's kernel — differ by at most 15 levels, mean 0.19, SSIM
0.9996, inside `corpus.rs`'s bound; the same page against HEAD's processor (the general stroker)
differs by the same amounts. Single marks agree to under a level, so the page's residue is where
marks overlap: the drawing lays its dots end to end with each round end over its neighbour, and the
two backends composite every one of them into eight-bit pixels with their own rounding. That is the
lanes' own difference (ADR 0082's relaxation), stated as a bound and not as identity.

**The corpus moves little, and where it moves is named.** The six arms against HEAD's
(`/home/AI/arms-1496/`, the oracle HEAD's general stroker): every arm exit 0, no page on one side
only, and three pages moved at 1× and two at 4× — `bug1978317.pdf`, `issue12295.pdf`,
`issue12810.pdf` — each by under a hundredth of a level of mean, all inside the bound; the first two looked at.
Against ADR 1829's oracle `issue12295.pdf` at 1×, which that round named as this lane's to close,
moves from 0.1063 to 0.1032 on `Cpu` and `Gpu` and from 0.0413 to 0.0464 on `Compute`: its thin rules
are now drawn by the same set on both sides, and what is left is the heart-rate trace — a joined
polyline neither ADR admits — and the glyphs' edges, which the difference map shows and nothing
here changes.
