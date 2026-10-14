# 1928 — A polygon in a shader is written by constant indices, so DX12's compiler builds it

Session 1546, slot 1 of batch eighty-seven. Status: **accepted, built**. Asked by the project owner
from Windows without `--cpu` (2026-10-15). Context: ADR 1492 (the oblique image's area), raster's ADR
0097 (the line lane), ADR 1929 (what a refused pipeline does). Code: `raster/crates/raster-gpu/src/
shaders/image.wgsl`, `shaders/line.wgsl`. It cites no clause and moves no ledger row.

## 1. What the owner's adapter said

`Device::create_render_pipeline`, label `raster image`: FXC's `D3DCompile` refused the fragment
stage with X3500, "array reference cannot be used as an l-value; not natively addressable", at
(141,17-48), and X3511, "forced to unroll loop, but unrolling failed", at (99,5-15). `wgpu` 30's DX12
backend writes HLSL with `naga` at shader model 5.1 and compiles it with FXC unless told otherwise.

Translated here with `naga` 30's `hlsl-out` under the backend's options (`restrict_indexing`,
`force_loop_bounding`), the two positions are `keep_side`'s `while(true)` and its second
`out_1.corner[min(uint(_e65), 7u)] = …`, seven lines further down in the owner's text than in ours
(the sampler heap's preamble differs). So the refusal is the polygon's append: a corner written at
`out.count`, inside a loop bounded by `polygon.count`. FXC cannot address an element of an array
held in a struct by a run-time index on the left of an assignment, tries to unroll the loop to make
the index constant, and cannot, because `naga`'s loop guard and the run-time bound give it no trip
count. A read by a run-time index is FXC's own selection among registers and drew no error.

## 2. The decision

**No corner is written through a computed index, and every loop over the corners runs to the
constant eight and leaves at the count.** `push_corner` writes each of the eight slots by a literal
index, keeping its value unless it is slot `count`; the area loop and the cut loop break at
`i >= count`. The arithmetic, its order and the `out.count < 8u` guards are what they were.

**`line.wgsl`'s polygon is the same construction and had the same defect** — `cut` appended
`out.v[out.count]` — so it is rewritten the same way (`push_vertex`). The owner's page had no thin
line the lane admitted on that frame; the next one would have refused the line pipeline in the
same words.

**DXC is not the answer.** `wgpu` can be told to compile with DXC, a C++ library loaded into the
unconfined host; principle 3 admits one named exception there and this is not it.

## 3. What it moved: nothing

- **The translation**: the rewritten shaders' HLSL, both entry points of each, carries no array
  write whose index is not a literal (`scratchpad/r1546/hlsl-probe`, kept out of the tree).
- **Byte for byte**: 612 frames per lane — 1 296 oblique images (48 placements, three filters, no
  clip, a rectangle and a residue) and 540 thin-line scenes (six widths, three caps, four
  transforms, three clips) — on each of Coverage `Cpu`, `Gpu` and `Compute`, on llvmpipe and on the
  Radeon 890M: 1 836 digests per adapter, HEAD's export against this round's, **none differs**. The
  instrument was calibrated by planting a halved area in both shaders: every image frame and 261 of
  the line frames moved.
- **The corpus**: the six arms against HEAD's at `/home/AI/arms-1546/`, page by page by digest, in
  the round's record.

## 4. What is not claimed

FXC has not compiled the rewrite: this machine has no FXC, and the claim rests on the translation
and on the errors' own wording. The run-time reads remain; if FXC refuses one, ADR 1929 makes it a
refusal said out loud and the processor draws the page.
