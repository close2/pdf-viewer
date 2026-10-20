# 2116 — A rectangle's converter is the page's to choose, so a tall page's first strip is the page

Session 1640, slot 2 of batch one hundred, a pixels round on ADR 2100 §4's find, under the project
owner's switch to code improvements and bug fixes of 2026-10-14. Status: **accepted** and **built**
in `render-cpu`. Clause: ISO 32000-2 §10.7.4 (the row stays `departed`, ADRs 1060, 1360, 2058; its
note gains a sentence). Context: ADRs 0219, 0476, 1348, 2058, 2084, 2100, 2106; traps 146, 213, 214,
216, 222. Code: `render_cpu::scan::rectangle_converter_reaches`, `scan::Scratch::for_page`,
`MaskCache::new`. Test: `render-cpu/tests/page_past_the_tile.rs`'s
`a_rectangle_in_a_tall_page_s_first_strip_is_the_page_s` (new, red on HEAD).

## 1. The premise, and the hypothesis that did not hold

**Premise (ADR 2100 §4): held in its shape, not in its figure.** `issue12810.pdf` at 4× is
6 912 × 10 368, and drawn in 2, 4 and 16 strips the planner cuts it once, at row 1 219, every
count: 7 150 pixels differ from the page drawn whole, at most 12 levels. The first strip, whose
origin is the page's, does move — but **172 pixels by one level**, not twelve. The 12 are below
the cut, and only five pixels move by more than three levels anywhere.

**Hypothesis — hatching strokes past 8191 for which `measurable` answers differently on the whole
page and on a strip — did not hold.** Isolated on the full-size target by the smallest mark (trap
213), with each drawing's surface and translation printed (trap 222):

- **The first strip's 172 pixels** are one command: a black fill, a 0.66-unit bar at `306.66–
  307.32`, at 4× the columns `1226.64–1229.28`. On the page drawn whole the surface is 10 368 rows,
  past `RECTANGLE_SURFACE_LIMIT`, so `intersected` wrote the closed form (ADR 2058); the first
  strip's surface is 1 219 rows, under it, so the same rectangle went to `tiny-skia`'s 8.8
  converter. Column 1229 holds 0.2799 of the bar: 184 drawn whole, 183 in the strip. A converter
  chosen from the surface's height — trap 146's shape exactly, on the rectangle converter.
- **The 12 levels** (pixels `(3007, 4291)`, `(3067, 4271)`, `(5203, 3199)`) are two contour
  strokes, 1.68 units wide, round-joined, each with one tight curve, so `folding_stroke_outline`
  states them as convex pieces (ADR 1348) and `mask_fill` gives the pieces to the library's
  supersampled converter on both drawings: surface 6 912 × 10 368 at `ty` 4 165.2 whole,
  6 912 × 9 149 at `ty` 2 946.2 below the cut. Same converter, same anti-aliasing, the origin moved
  by 1 219 rows across the binade at 4 096 — ADR 0219's shifted origin flipping one of sixteen
  samples, 11–12 levels of this green on white. That is the pieces' sixteenths, which slot 5 of
  this batch owns (ADR 2106 §3), not a decision taken from the surface.

## 2. Decision

The converter a rectangle takes is decided by **the page**: `Scratch`, which every `MaskCache`
already carries one of per band, is made by `Scratch::for_page` with the page's width and height
from the cache's `Surface`, and `rectangle_converter_reaches` asks that extent as well as the
surface's. On a page past 8191 every surface — the first strip, a strip under the limit, a group's
— writes the closed form, as the page drawn whole does; on a page inside it nothing changes. The
surface is still asked, so that no surface past the limit reaches the library's tiler whatever page
it belongs to. A `Scratch` made without a page (the tests in `scan.rs`) takes its surface as the
page, which is the old answer.

**What is left stated, not removed**: a *window* of a page past 8191 (ADR 1930) is a target of its
own, under the limit, and keeps the library's 8.8 converter — ADR 2058 §2.3's one level between a
page and its window, unchanged here.

## 3. Measured

On the round's export of HEAD plus this round's change (ADR 1890), against an export of HEAD:

- **The witness at 4×**: drawn whole, 0 pixels from HEAD's. In strips, 7 150 → 6 978 differing at
  2, 4 and 16, the first differing pixel now `(1201, 1271)`, below the cut; the first strip 172 → 0.
  The worst stays 12, at the folding strokes above.
- **`page_past_the_tile.rs`**'s new test: a 64 × 8 256 page, the witness's bar, cut at
  `[0, 1000, 8256]` and `[0, 100, 4000, 8256]` — 39 pixels of 184 against 183 on HEAD, 0 here.
  Its two cut tests of a wide and a tall page passed on HEAD because their rectangles are dyadic,
  which the two converters state alike (trap 13).
- **Beside slot 5's measure of the pieces** (ADR 2122), on an export of HEAD plus both rounds'
  `render-cpu`: the witness in strips differs at 7 000 pixels, **worst 3**, the first strip 0 — the
  twelve were the pieces' sixteenths, as §1 found, and what is left is ADR 0219's shifted origin
  below the cut.
- **What it costs**: the witness at 4×, one draw on one thread under `callgrind_rasterise`,
  16 582.43 → 16 583.33 million instructions (+0.005%) — one comparison more per rectangle.
- **`raster_golden`** draws each page in one strip, where the surface is the page, so this half
  moves no golden page; the moved pages are ADR 2117's.
- **`window_against_whole`** on the owner's witness at 1.25⁸ and 1.25¹⁰, 1 200 × 900, HEAD and this
  interleaved in one `--clock` hold: every whole page's and window's digest HEAD's (1 736 and 5 478
  of the window's pixels from the page's, at most 2 levels, both builds). The page is 11 176 wide
  and took the closed form before and after.
