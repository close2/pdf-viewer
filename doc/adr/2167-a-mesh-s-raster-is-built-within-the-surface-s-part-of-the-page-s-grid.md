# 2167 — A mesh's raster is built within the surface's part of the page's grid

Session 1665, slot 3 of batch one hundred and three, the second decision of the pixels round ADR 2166
opens. Status: **accepted** and **built** in `pdf-render` and `render-cpu`. Clause: ISO 32000-2
§8.7.4.5.5 and §10.7.4 (the §10.7.4 row stays `departed`; its note's last sentence names this ADR).
Context: ADRs 0051, 1217, 1259, 2133, 2148 §3, 2166; traps 213, 225, 228. Code:
`pdf_render::MeshRaster::{build, build_within}`, `render_cpu::shading::fill_mesh`. Tests:
`pdf-render`'s `a_raster_within_a_region_is_the_whole_raster_there`, and
`render-cpu/tests/window_on_the_page_grid.rs::a_window_s_mesh_is_the_page_s`, failing on the tree
before.

## 1. What did not move, and why it could not

ADR 2148 §3 left a mesh's raster on the surface's grid: `MeshRaster::build` painted from its target's
origin, so on the page's grid a window's raster would have held every pixel above and left of the
window too, and its triangles were mapped by the surface's matrix, the corner composed in. §8.7.4.5.5
colours a pixel by the triangle holding its centre, and a vertex mapped at the window's magnitude is
the same real number rounded in another binade (trap 228), so a centre within an `ulp` of an edge,
and a colour within one of a rounding step, went another way in the window than on the page.

## 2. Decision: the raster takes a region of the grid

`MeshRaster::build_within(…, to_device, (left, top, right, bottom))` rasterises the mesh into the part
of that rectangle of `to_device`'s grid it covers, its `left` and `top` on the grid; `build` is the
same call over `(0, 0, width, height)`, so `render-raster`'s caller and every byte it draws are
unchanged. `render_cpu::shading::fill_mesh` is handed the mesh's `Placed`, maps the triangles and
tessellates the patches by the page's matrix, asks for the surface's part of the page's grid, and
moves the raster's corner onto the surface — `shading::OnSurface`, the construction ADR 2148 gave the
cone and the washed shading. Where the surface is the page the region is the target and the matrix is
the page's, so a page drawn whole is drawn as it was.

## 3. Measured

- **The tests**: `a_window_s_mesh_is_the_page_s` failed on HEAD at 8 pixels of the second window, one
  level each — on a target moved by 0.3716 of a pixel across, whose nearest 2⁻¹⁰ and 2⁻¹¹ differ, so
  the page's vertices and the window's round apart (a translation of zero across hides it: the first
  form of the test passed on HEAD and was rebuilt, trap 13) — and passes here, as does the region
  test in `pdf-render`.
- **The six corpus arms** of an export of HEAD plus this round against `/home/AI/arms-1663/`: **0
  frame digests differ** in any arm (968 × 3, 965 × 3 pages) and the mean error against the oracle
  moves on **no page** at 1× or 4×; `raster_golden` 974 held, 0 moved, and its division census is the
  tree's before, 29 pages at most 383 pixels and 3 levels (ADR 2166 §4). Eight of the nine first
  pages past the range at 4× (ADR 2166 §1's census; the ninth, `issue19517.pdf`, is past every gate's
  pixel budget) drawn whole and in 16 strips by both builds: 0 pixels apart
  between the builds, and 0 between whole and strips in each. `issue12810.pdf`'s knockout group, the
  one the census found, lies at rows 8 154 to 8 944 at 4× and still moves nothing: each of its 13
  elements is clipped, so `encode` draws it as ADR 1320's two stages, `DestinationOut` then `Plus`,
  whose coverage `crate::area` measures — no element of it reaches the library's `Source`, which is
  what the range decides (a census of knockout *groups* over-counts the marks the range reaches; the
  probe printing each fill's blend mode is what said so).
- **The owner's witness**, `window_against_whole`, both builds interleaved in one `--clock` hold (load
  3.3 to 4.5), 1200 × 900, four threads, two passes of two rounds: at 1.25⁸ the window parts from
  the page at 0 pixels in both, the page's digest and the window's the same in both builds; whole
  2289, 2145, 1910, 1725 → 2103, 2061, 1774, 1769 ms, window 248, 240, 219, 225 → 233, 255, 197, 193
  ms. At 1.25¹⁰ the window is 0 pixels from the page in both and its digest the same, and **the page's
  digest moves** (`c97e3baa…` → `a8b9c27e…`): whole 5592, 4820, 4775, 4312 → 4970, 4883, 4431, 4461
  ms, window 244, 259, 253, 219 → 273, 276, 218, 250 ms, inside the spread.
- **What moved on the witness, isolated** (trap 213): drawn by `CpuRasterizer` at the same target
  with one strip, the page is the same in both builds to the byte; at 4 and 16 strips HEAD parts from
  it at **4 pixels, one level each**, at x 8 760 to 8 948 — past the range — and this round at 0. A
  build of this round with `fill_rectangles` alone handing `intersected` the surface's grid again
  reproduces HEAD's 4: a rectangle past the range is measured by `crate::area` rather than by the
  closed form, and `fill_rectangles` measured it on the strip's grid, a seventh route ADR 2133's list
  and ADR 2148's left out (trap 231). The pixel at (8 786, 5 315) is the top row of the bar of a
  glyph "1" in "12. FIRE R…", 0 on the page and 1 in HEAD's strips; looked at, it is the page's text,
  drawn as it was.
