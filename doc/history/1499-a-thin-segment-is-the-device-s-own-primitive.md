# 1499 — A thin segment is the device's own primitive

Slot 4 of batch seventy-nine, 2026-10-10, a pixels round on the owner's performance target. ADRs 1835 and
raster's 0097; no question. No ledger row moved: the clauses are read as they stand.

**Premise** held in the code: every one-segment stroke was its own uploaded outline, flattened, expanded
and scan-converted on the host. **Hypothesis** held in direction, not in its numbers: the instance is
60 B, not 32 (endpoints, pen, cap, colour, and the clip's rectangle a round end needs per pixel), and the
pre-count is 64 B a line, not half of 96, because a line off the closed form writes a quad.

**Built.** Raster's `Command::Line` (two points, width, cap, solid colour, no outline) and its lane:
the exact area in the frame where the pen is the unit disc, any direction under a transform that keeps
the axes — a clipped band's shoelace area, each round end the disc in the pixel cut by the endpoint's
line, as triangles and sectors. A residue clip or a turning transform draws the two-point stroke, to the
bit. `render-raster` sends what `pdf_render::thin_segment` (ADR 1829) admits as one line and consumes the
view (ADR 0702). Slot 1 widened its classification to diagonals mid-round; the shader went general.

**The figure moved** (headless, RADV, one clock hold, HEAD and after exports alternating, `Cpu` /
`Compute`): the witness's device frame at the fit 9 666 / 9 755 ms → **1 204 / 1 221 ms** (scene 2 560 →
327, encode 6 910 → 672), 32 791 resources where every stroke was one. Generator chop 8 9 796 / 9 832 →
1 119 / 1 278 ms; chops 12 and 16 refused → drawn; chop 24 (12 185 819 commands) drawn in 2 902 /
4 026 ms; chop 1 admits no mark and draws the same bytes. No owner's-display figure: the contract names none.

**Agreement.** 1 944 lines against the set's `f64` polygon: worst 0.594 levels (RADV), 0.500 (llvmpipe);
planted defects 232 and 152. The witness against ADR 1829's oracle: ≤ 15 levels, mean 0.19, SSIM 0.9996,
the residue overlapping dots composited at eight bits by each side. `issue12295.pdf` 1× against it:
0.1063 → 0.1032 (`Cpu`, `Gpu`), 0.0413 → 0.0464 (`Compute`); the rest is its joined trace and glyphs.

**Files.** `raster/crates/raster-scene/src/{error.rs, scene/{command,builder,validate,cost}.rs}`;
`raster/crates/raster-gpu/src/{encode.rs, encode/{line,instance,encoded,opacity,residue,parallel}.rs,
encode/parallel/commit.rs, frame.rs, compose.rs, compose/draw.rs, device/{record,staging}.rs, pipeline.rs,
pipeline/spec.rs, shaders.rs, shaders/{copies,shape_inputs}.rs}`, `shaders/line.wgsl` and six shaders'
copy comments, `tests/line_lane.rs`; `raster/doc/PLAN.md`, raster ADR 0097; `crates/render-raster/src/
stroke.rs`, `examples/heavy_drawing.rs`; ADR 1835, `doc/todo/69`, this record.

**Gates.** `rustfmt --check`: exit 0. `clippy -D warnings --all-targets` on `raster-scene`, `raster-gpu`,
`render-raster`: exit 0. `nextest` on the three: 868 passed, 5 skipped. `cargo test -p conformance`: exit
0, then 101 on neighbours' records 1497, 1498, 1500 over forty lines. Six arms against `arms-1496`,
own target directory: all exit 0, 3 pages moved at 1× and 2 at 4×, inside the bound, two looked at.
`turn_path` (`--clock`), HEAD and after: exit 0, 33 banded, 0 outside. Ladder and witness, both lanes: two clock holds, exit 0. Duration 10 400 s.
