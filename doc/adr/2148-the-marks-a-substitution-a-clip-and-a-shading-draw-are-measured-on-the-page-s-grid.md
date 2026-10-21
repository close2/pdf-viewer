# 2148 — The marks a substitution, a clip and a shading draw are measured on the page's grid

Session 1656, slot 2 of batch one hundred and two, a pixels round on ADR 2133's last section, under
the project owner's switch to code improvements and bug fixes of 2026-10-14. Status: **accepted**
and **built** in `render-cpu`. Clause: ISO 32000-2 §10.7.4 (the row stays `departed`; its note's
last sentence names this ADR and ADR 2149). Context: ADRs 0219, 2058, 2084, 2106, 2132, 2133, 2149;
traps 213, 214, 216, 222, 225, 228. Code: `fill::draw_fill`'s marks and bands,
`stroke::draw_stroke`'s dots, `stroke::draw_long_mitres`, `stroke::sub_pixel_rule`,
`mask_cache::state_regions` and `scan::{mask_fill, mask_union, mask_intersect}`,
`shading::{fill_radial, fill_with_background, fill_mesh, fill_with_raster, OnSurface}`. Test:
`render-cpu/tests/window_on_the_page_grid.rs`, five new, each failing on the tree before.

## 1. Measured first, and the premise

ADR 2133 drew a window of a page on the page's grid and named, in its §5, the routes that still
took the surface's matrix. On HEAD the owner's witness at 1.25⁸ of the fit parted from its page at
**99 pixels, at most one level** (`window_against_whole`, 1200 × 900, four threads): 97 under a
stroke, 2 under a fill. Isolated by the smallest mark on the full-size target (trap 213), the most
different pixel is held by one command, a zero-length stroke under a round cap whose device width is
1.44 pixels — a dot past the two by two pixels `thin_segment` paints in closed form, filled by
`split_degenerate`'s circle under the surface's matrix. That is the sixth route of ADR 2133's list,
which the brief's five folded away; **the premise held, and the route that carried the residual was
the one not named**.

§10.7.4 identifies a pixel by its device coordinates alone — "Pixel boundaries always fall on
integer coordinates in device space" — so a window's pixel owes what the page drawn whole has there,
and nothing in the clause lets the answer depend on where a surface begins.

## 2. Decision

**Every mark one of these routes draws is decided and measured on the page's grid, and only its
write moves by the surface's corner** — `ToDevice::placed`, as ADR 2133 states it, handed to the
converter that draws the mark:

- **§10.7.4's marks for a subpath with no area** (`pdf_render::split_collapsed_fill`) and **its bands
  for a shape thinner than a pixel** (`pdf_render::sub_pixel_bands`), for a fill and for a stroke's
  rule: the pixel lines are found under the page's matrix, stated back through its inverse, and
  filled by `scan::fill_stated`, which `crate::area` measures on the page's grid.
- **A rule under one level of a pixel**, widened to one pixel at the alpha its width implies, and
  its caps; **§8.5.3.2's dot** where it is not `thin_segment`'s; **a long mitre's wedges** with
  the bevelled outline they complete: each a `fill_stated` of the path the construction built.
- **A clip's own mask**: the region is `pdf_render::clip_region`'s under the page's matrix, its
  rectangles `rectangular_mark` on the page's grid moved onto the band, and `scan::mask_fill` takes
  a `Place`, so `crate::area` measures the clip where the page measures it. A band is a surface
  with a corner on every page, so this moves pages drawn whole too (§3).
- **A shading drawn as a raster**: a cone (§8.7.4.5.4) and a shading washed with its `/Background`
  are evaluated at the page's own pixel centres over the surface's part of the page
  (`shading::OnSurface`), and the raster is placed at its corner less the surface's; the shape's
  edge is `scan::fill_positioned`'s, measured on the page's grid with the paint positioned on the
  surface already.

Where the surface is the page — one strip, no band — every call is the call it was: the page's
matrix and the surface's are the same numbers, and the corner is `(0, 0)`.

## 3. What does not move, and why

- **A mesh's raster** (`pdf_render::MeshRaster::build`) paints from its target's origin, so on the
  page's grid a window's raster would hold every pixel above and left of the window too; its
  triangles keep the surface's matrix and its edge is the page's. A region of the page's grid for
  `MeshRaster::build` is `pdf-render`'s, and is what would move it.
- **A sampled shading's `/Background` raster**: `Shading::sampled_at` crops its cells to the target
  it is handed, the same cost; it keeps the surface's grid, by name in `fill_with_background`.
- **A smoothed image's samples**, read by the library's pattern under the surface's matrix; ADR
  2100 moved the unsmoothed one.
- **The library's stroker**: ADR 2149.

## 4. Measured

On exports of HEAD and of HEAD plus this round (ADR 1890), each its own target:

- **The tests**: the five new ones failed on HEAD — 16 (shading), 8 (dots), 16 (clip), 2 (marks)
  and 1 (bands) pixels in the first window that parted, by one level and on one shaded pixel two — and pass here; a sixth, a `0 w` stroke drawn aliased, passed on HEAD as well
  — it reaches `draw_stroked_outline`, not the library's stroker — and was dropped as testing
  nothing this changes (trap 13).
- **The window against the page**, `window_against_whole` of both builds interleaved in one
  `--clock` hold (load 6.2 to 6.3, a neighbour's outside the lock), 1200 × 900, four threads, two
  runs each: at 1.25⁸ **99 → 0** pixels, the window landing at 191, 223 → 208, 203 ms and the page
  at 1716, 1938 → 2011, 1636 ms; at 1.25¹⁰ 0 → 0, the window's digest the same in both builds, at
  271, 272 → 263, 262 ms. Inside the spread.
- **`raster_golden`**: 968 held, **6 moved**, every one by one level — `issue1905.pdf` 550 pixels on
  one row, the bottom edge of a clipped panel; `issue840.pdf` 32 on one row; `issue18529.pdf` 11 on
  the top row of a shaded frame; `personwithdog.pdf` 4; `bug1703683_page2_reduced.pdf` and
  `bug1721218_reduced.pdf` 1 each — drawn by both builds and looked at (trap 1): the same pages,
  the moved pixels the edge rows of clipped marks. The TSV is regenerated on those 6 lines. Its
  division census at 2, 4, 8 and 16 strips: **60 → 29** pages moving at some division, at most
  **513 → 383** pixels, 3 levels (`blendmode.pdf`, ADR 1758). Of the eleven pages its list held
  past the bound, the eight ADR 2133 released and `issue7014.pdf` (now 0 pixels at every division)
  and `pdfjs_wikipedia.pdf` (383, one level) leave it; `blendmode.pdf` stays, and the census passes
  with that one name.
- **The six corpus arms** against `/home/AI/arms-1655/`: **0 frame digests differ** in any arm
  (968 × 3, 965 × 3 pages) — the lanes they digest are not this crate's — and the mean error
  against the oracle, which is this backend in strips, moves on 5, 5 and 4 pages at 1× by at most
  0.0017 and on none at 4×. The six pages drawn by both builds in strips and looked at:
  `issue7014.pdf` 235 pixels, at most 2 levels, the edge rows of its clipped highlight and strike
  marks; `pdfjs_wikipedia.pdf` 107 at 1, its link underlines; `issue18529.pdf` 11; `issue16263.pdf`
  5; `personwithdog.pdf` 4; `tracemonkey_a11y.pdf` 1.
- **The guards** (trap 214): `hostile_geometry` 4 passed, `fixed_documents --ignored` 1 passed.
