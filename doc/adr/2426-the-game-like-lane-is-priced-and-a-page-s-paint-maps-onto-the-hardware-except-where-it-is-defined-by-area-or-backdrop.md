# 2426 — The game-like lane is priced, and a page's paint maps onto the hardware except where it is defined by area or by backdrop

Status: proposed — a measurement and a verdict on the owner's two questions; nothing is built into
the product, and building the lane is Q662's.
Session 1796, 2026-10-26, a "pixels" round added to batch 119 at the project owner's word. Context:
ADR 0368 (a tessellation lane refused), ADR 0698 (the compute rasteriser priced), ADR 0702 (the
scene in page space), ADRs 2315 and 2355, ADR 2408 (the zoom step's blurry frame); raster's ADRs
0016, 0070, 0076, 0079 and 0080. The argument and every figure: RFC 0013. ADR 2427 is unused.

## 1. The question

The owner, 2026-10-26: "but why don't we send everything to the GPU in a way, that it's GPU
elements? Games also don't need to send everything again, whenever something zooms?" and "or are
the PDF paint instructions completely incompatible to GPU painting instructions?" ADR 0368 refused
a lane that tessellates into coverage tiles and ADR 0698 priced a compute rasteriser; nothing had
priced the lane a game uses — the page's triangles resident, the hardware rasteriser drawing them
straight into the frame under MSAA, the view one uniform.

## 2. What was decided

1. **The lane is priced by a probe that stays an example**: `crates/render-raster/examples/
   resident_triangles/`, stencil-then-cover fills with Loop–Blinn curve triangles evaluated per
   sample, strokes outlined once in page space, images as textured quads, one render bundle. It is
   in `render-raster` rather than under `raster/` because it reads documents and raster never sees
   a PDF byte (raster's `CLAUDE.md`); it draws its marks against `render-cpu` drawing the same marks
   (`RESIDENT_ORACLE`) under the corpus gate's three bounds, so a later round can re-price it on
   another adapter by running it.
2. **The figures stand as the lane's price** (RFC 0013 section 4, Radeon 890M, one clock hold at the
   middle sclk line): a zoom step is 0.87–1.15 ms on ISO p. 101 against the compute lane's 1.2–2.6,
   1.27–1.49 ms on a two-page column, 1.30–2.38 ms on the plan sheet against 5.6–39.7. The brief's
   hypothesis held on the dense page in direction and not in size (0.37 to 0.80 of the compute
   lane's step), and its premise did not hold on the heavy page, whose step is image samples
   crossing again, not fills.
3. **The answer to the second question is "no, not incompatible"**, with the line drawn where the
   specification draws it: fills under both rules, curves, glyphs, strokes outlined in page space,
   solid and constant-alpha paint, images (§10.7.4's pixel-centre rule is the rasteriser's own),
   shadings and clips map onto hardware primitives; §8.5.3.3's coverage as area, §10.7.4's
   no-disappearance for a path, backdrop-reading compositing (§11.3.5's other blend modes, §11.4's
   groups, §11.6.5's soft masks) and byte-identical output across adapters do not. A later round
   does not re-argue the table in RFC 0013 section 6 without a clause that moves a row.
4. **Nothing is built until Q662 is answered.** The recommendation there takes the resident image
   texture first, on its own, and the hardware lane only as the moving view's, with the compute
   lane's exact frame at rest.

## 3. What it costs not to build it

The blurry stand-in of ADR 2408 stays on every step whose sharp frame misses the refresh; the column
and the plan sheet keep missing it. Nothing measured here moves a gate, a pixel or a figure the tree
holds.
