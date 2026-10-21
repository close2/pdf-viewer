# 2166 — The library's range is asked of the page, so a window draws a knockout element as the page does

Session 1665, slot 3 of batch one hundred and three, a pixels round on ADR 2149 §3's bound, under the
project owner's switch to code improvements and bug fixes of 2026-10-14. Status: **accepted** and
**built** in `render-cpu`. Clause: ISO 32000-2 §10.7.4 (the row stays `departed`; its note's last
sentence is rewritten). Context: ADRs 0219, 0226, 0269, 2058, 2116, 2133, 2148, 2149; traps 146, 213,
214, 216, 225, 228, 231. Code: `scan::{library_reaches, Exact::usable_on, fill_as, fill_rectangles,
intersected, stroke, mask_fill}`. Tests: `render-cpu/tests/window_on_the_page_grid.rs`'s
`a_window_s_knockout_element_takes_the_range_the_page_takes` and
`a_strip_of_a_tall_page_draws_a_knockout_element_as_the_page_does`, each failing on the tree before.

## 1. The premise, checked

ADR 2149 §3 measured it: a mark the library draws itself — every mark under
`compose::carries_coverage_as_alpha`'s `false`, a §11.4.6 knockout element above all — has its
anti-aliasing withdrawn by `scan::expressible` where its coordinates pass `tiny-skia`'s 8191-pixel
range, and the question was asked of the **surface**. `grep -n 8191 crates/render-cpu/src` and the
callers of `expressible`, `measurable` and `Exact::usable` give the places it was asked: `fill_as`'s
library branch, `fill_rectangles` (which also handed `intersected` the surface's grid), `intersected`'s
choice between the closed form and `crate::area`, `stroke`, and `mask_fill`. The rectangle converter's
own surface rule was already the page's (ADR 2116); nothing else asks the range.

**The census first** (`pdf-model/examples/library_range_census`, the pdf.js corpus's first pages at the
corpus gates' two scales): 963 documents open; **1 page is past the range at 1×** (`issue19517.pdf`,
past every gate's pixel budget) and **9 at 4×**, of which **one holds a knockout group**:
`issue12810.pdf`, 6 912 × 10 368, one group of 13 elements. **The hypothesis did not hold**: those 13
are clipped and drawn as ADR 1320's two stages, which `crate::area` measures, so no corpus page at the
gates' scales holds a mark the range decides, and none moves (ADR 2167 §3).

## 2. Decision: the page's grid decides, and the surface is asked as well

`scan::library_reaches` answers whether the library anti-aliases a mark: inside the range on the
page's grid **and** on the surface. Every place in §1 asks it, or `Exact::usable_on(corner)` for a
mark already stated as rectangles on the surface, and `fill_rectangles` and `stroke` are handed the
mark's `Placed` rather than the surface's matrix alone, so `intersected` measures a rectangle past the
range on the page's grid as the page drawn whole does.

- **Why the page**: §10.7.4 identifies a pixel by its device coordinates alone — "Pixel boundaries
  always fall on integer coordinates in device space" — so a strip's, a band's or a window's pixel
  owes what the page drawn whole has there, and the range is a property of where the mark lies on the
  page, not of where a surface happens to begin (trap 146's rule, ADR 2116's construction).
- **Why the surface as well**: its coordinates are what the library is handed, and outside the range
  they are what its arithmetic cannot hold — ADR 0269's abort. The two answers part only where a
  window's corner lies more than 8191 pixels beyond a mark's near side; there the library is kept
  inside its range and the window keeps the difference, by name in `library_reaches`.
- **Where the surface is the page** — one strip, no band — the two questions are one, so a page drawn
  whole is drawn as it was.

## 3. The alternative, and why not now

ADR 2149 §3 named the other remedy: draw such marks through `crate::area`, so that the page is
anti-aliased too. It is not this round's, for one reason a later round must answer first: the
library states a knockout element as Porter-Duff `Source` with its coverage applied by its own lerp,
and `intersected` delivers coverage as a mask the `Source` pipeline multiplies into the paint — the
partly transparent pixel ADR 0226 forbids. A coverage route for `Source` needs its own composition
(`lerp(dst, src, shape)`, the shape the clip's `min` with the coverage), which is a construction, not
a range rule. **The cost this decision accepts**: a window or a strip of a page past 8191 pixels now
draws a knockout element aliased where it was anti-aliased — one corpus page at the gates' scales,
and every page the viewer magnifies past the range.

## 4. Measured

On exports of HEAD and HEAD plus this round (ADR 1890), each its own target:

- **The tests**: the two new ones failed on HEAD — 644 pixels in the window, 864 at two strips — and
  pass here.
- **`raster_golden`**: 974 held, 0 moved; its division census 29 pages, at most 383 pixels and 3
  levels, 1 named past the bound — the figures before. No first page at 1× the gate draws is past the
  range.
- **The guards** (trap 214): `hostile_geometry` 4 passed; `fixed_documents --ignored` 1 passed, 77
  checked.
- **The arms, the window and the witness**: ADR 2167 §3, which measured both changes in one export —
  no corpus page moves at the gates' scales, and the owner's witness at 1.25¹⁰ drawn in strips moves
  4 pixels by one level onto the page drawn whole, a rectangle's route this decision's `Placed` reached.
