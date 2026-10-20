# 2133 — A mark is measured on the page's grid, and only its write moves

Session 1648, slot 2 of batch one hundred and one, the second decision of the pixels round ADR 2132
opens. Status: **accepted** and **built** in `render-cpu`. Clause: ISO 32000-2 §10.7.4 (the row
stays `departed`; its note gains the sentence ADR 2132 gives it). Context: ADRs 0219, 1082, 1348,
1758, 2100, 2106, 2132; traps 13, 146, 213, 214, 216, 222, 225. Code: `render_cpu::scan::{Place,
fill_stated, fill_positioned, Exact::moved}`, `area::Region`'s corner, `surface::{Placed,
ToDevice::placed, on_surface}`, `fill::draw_fill`, `stroke::draw_stroked_outline`,
`image::centres::draw`. Test: `render-cpu/tests/window_on_the_page_grid.rs`'s
`a_window_s_fills_and_strokes_are_measured_from_the_page_s_vertices` and
`a_window_s_image_takes_the_sample_its_centre_maps_to_on_the_page`.

## 1. The hypothesis, and the half of it that did not hold

**The brief's hypothesis held in its mechanism and not in its remedy.** What was left of a window
once ADR 2132 drew it on the page's grid was ADR 0219's shifted origin, as the brief said — but not
on the library's path converter. Every mark the route printed was measured by `crate::area`, and
the remedy the brief proposed, the mark's vertices stated on the page's grid with the corner taken
off before the converter saw them, left 21 of a 101 × 90 window's pixels a level apart in this
ADR's own test. `area` evaluates each crossing as `upper.x + (above − upper.y) · slope` and each
row's height as a difference of absolute heights: sums at the vertices' magnitude, which moving the
vertices moves. ADR 0219 called this residue the dependency's arithmetic and out of reach; on this
converter it is this crate's own arithmetic, and in reach.

## 2. Decision

**A mark's coverage is measured on the grid its matrix maps onto — the page's, for a strip or a
window — and only the write into the surface moves by the surface's corner.** `ToDevice::placed`
states a mark as `Placed`: the surface's matrix (what a paint is positioned by, and what a bound on
the surface is read from), the matrix onto the page's own grid, and the surface's corner there.
`scan::Place` carries the three to the converters:

- **`crate::area`** is handed the page's matrix, and its `Region` the corner: the region is counted
  in page columns and rows, clamped to the surface's part of them, every crossing is evaluated at
  the magnitude the page drawn whole evaluates it at, and only the index the coverage is written at
  moves (`Region::on_surface`). The range `area` measures inside (`MEASURED_LIMIT`, ADR 2058) is
  asked of the page's coordinates, as the page asks it.
- **The library's converter**, whose fixed point is relative to the pixmap it draws into, is handed
  the page's own `f32` vertices less the corner — exact, as `tiny-skia`'s own tiler moves a path
  onto a tile — and no matrix, the paint moved first as `fill_path` would move it
  (`surface::on_surface`).
- **A rectangle's closed form** is stated on the page's grid and moved by the corner
  (`Exact::moved`), for a fill (`draw_fill`) and for a stroke's rectangular outline.
- **An unsmoothed image's square** takes the same route with its paint positioned on the surface
  already (`fill_positioned`); its samples were the page's since ADR 2100.

Where the surface is its own grid — a page drawn whole, in one strip — `Place::surface` is the
matrix the backend has always handed over, and every call is the call it was.

**A strip is the same construction**, its corner `(0, top)`: the shifted origin ADR 0219 measured
and ADR 1758 bounded is removed from the marks these routes draw on every divided page too, which
`strip_parallelism` and the census measure (§4). What keeps it elsewhere is named in §5.

## 3. What it costs, and why it is cheap

Nothing is copied for `area`, which maps each vertex itself: it is handed another matrix. The
library's route clones and maps the path once, which is what `fill_path` does with a matrix anyway;
it now does it here and draws under none. Per fill and per stroked outline one more
`Transform::then` composes the page's matrix beside the surface's. Figures in §4.

## 4. Measured

On exports of HEAD and of HEAD plus this round and ADR 2132 (ADR 1890), each its own target:

- **The window against the page**, `window_against_whole`, the runs of ADR 2132 §1: the owner's
  witness at 1.25¹⁰ 5 478 → **0** pixels (ADR 2132 alone 4 084; this without the rectangles' move
  9, every one under a rectangle's closed form), `issue12810.pdf` at 4× 811 → **0**. The window is
  the page's to the byte on both.
- **The division census at 1×** (`stroke_cut_census --draw`, every tracked first page drawn whole
  and in 2, 4 and 16 strips, both builds in one hold): pages moving at some division **181 → 58**,
  the most pixels 984 → 513, the worst 3 → 3 levels (`blendmode.pdf`, a blend function steeper
  than one, ADR 1758), past one level 6 → 2, every moved pixel summed 26 556 → 3 581. `raster_golden`'s
  own census, at 2, 4, 8 and 16 strips: 60 pages, at most 513 pixels and 3 levels, 11 named past the
  bound of which 8 may now leave it (the list is the merge's to shorten); it passes.
- **The division census at 4×**, the same instrument: **310 → 25** pages, the most 10 317 → 6 282
  pixels, the worst 12 → 3, past one level 11 → 3, summed 356 810 → 15 590. **No page moves by more
  than it does on HEAD at any division**, at either scale.
- **The page drawn whole in one strip** (`raster_golden`): 949 held, **18 moved**, every one by one
  level at 1 to 273 pixels — marks under a clip, whose band is a surface with a corner of its own
  (ADR 2106), now measured where the same mark without the clip is. `issue17730.pdf` (273) and
  `issue1905.pdf` (86) looked at beside HEAD's (trap 1): the same page, the moved pixels the edge
  row of link underlines and of a bar. `raster_golden.tsv` is regenerated on those 18 lines.
- **What it costs**, `callgrind_rasterise`, one draw on one thread, HEAD → this: the owner's witness
  at the fit 81 858.6 → 81 950.9 million instructions (+0.11 %), `issue12810.pdf` at 4×
  16 581.6 → 16 680.8 (+0.60 %), ISO 32000-2 page 101 twenty draws 4 765.4 → 4 777.0 (+0.24 %).
- **The clock**, `window_against_whole` of both builds interleaved in one `--clock` hold (load 1.5 to
  3.0), 1200 × 900, two runs each, HEAD → this, milliseconds to the window's landing: at four
  threads 195, 190 → 212, 212 (witness 1.25⁸), 244, 231 → 210, 264 (1.25¹⁰), 106, 69 → 69, 71
  (`issue12810.pdf` 4×); at twenty-four 125, 116 → 105, 108; 158, 147 → 142, 149; 77, 77 → 72, 71.
  Inside the spread. At 1.25⁸, a page under the range drawn in strips, the window parts from the
  page at 1 736 → 99 pixels (at most 1), which are §5's routes; at 1.25¹⁰ and on `issue12810.pdf`
  it is 0 at both thread counts, the window's digest one at both.
- **The six corpus arms** against `/home/AI/arms-1647/`: 0 frame digests differ in any arm (968 ×3,
  965 ×3 pages) — the lanes they digest are not this crate's — and the mean error against the
  oracle, which is this backend in strips, moves on 18, 20, 22, 15, 13 and 18 pages by at most
  0.0014, both ways. The three largest (`issue1350.pdf`, `bug1743245.pdf`, `issue7020.pdf`) drawn
  at 1× by both builds whole and in sixteen strips: the page whole is the same bytes in both, HEAD's
  sixteen strips parted from it at 538, 417 and 99 pixels, and this build's at none — the oracle is
  now the page drawn whole there.
- **The guards** (trap 214): `hostile_geometry` 4 passed — ADR 0269's spike is measured on the
  grid's coordinates, which `MEASURED_LIMIT` asks of exactly as on the page — `fixed_documents
  --ignored` 1 and `strip_parallelism` 2 passed.

## 5. What is left

- The marks drawn by routes other than these — `sub_pixel_bands`' bands, `split_collapsed_fill`'s
  marks, a clip's own mask (`mask_cache`), the library's stroker for a hairline (`scan::stroke`), a
  shading drawn as a raster, a dot's disc outside two by two pixels — still take the surface's
  matrix, and on a strip or a window round at its magnitude, as before. Each is one call site of the
  same shape, for a later round, measured first.
- A rectangle whose path is not rectangles on the device grid is not affected.
