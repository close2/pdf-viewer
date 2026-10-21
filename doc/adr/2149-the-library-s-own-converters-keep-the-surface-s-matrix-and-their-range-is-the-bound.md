# 2149 — The library's own converters keep the surface's matrix, and their range is the bound

Session 1656, slot 2 of batch one hundred and two, the second decision of the pixels round ADR 2148
opens. Status: **accepted** — a bound stated, nothing built. Clause: ISO 32000-2 §10.7.4 (the row
stays `departed`). Context: ADRs 0219, 0226, 1102, 2058, 2133, 2148; traps 13, 213, 216. Code:
`render_cpu::scan::stroke`, `scan::fill_as`'s library branch, `compose::carries_coverage_as_alpha`.

## 1. What reaches the library's stroker

`scan::stroke` is `tiny_skia::PixmapMut::stroke_path` under the surface's matrix, and since ADR 1102
it is the last of four roads: a stroke reaches it only where `draw_sub_pixel_rule`,
`draw_long_mitres` and `draw_stroked_outline` all decline — where `carries_coverage_as_alpha`
answers `false` (anti-aliasing withdrawn, or §11.4.6's knockout element under the library's
`Source`) **and** the width is at or under one level of a pixel, where the transform states no
thinnest line, or where the stroker refuses the path. A `0 w` stroke is one device pixel wide
(§8.4.3.2, `Stroke::device_width`) and takes the outline, not this.

## 2. Decision: it keeps the surface's matrix, by name

Its vertices could be moved as the outline's are — the page's vertices less the corner, under no
matrix, with the hairline's coverage restated as a width the library reads back exactly. It is not
done, for two reasons a later round must not re-litigate without new evidence:

- **The library's hairline is not a function of the vertices alone.** It clips each line to its
  pixmap in floating point before its fixed point sees it, and subdivides a curve at the vertices'
  magnitude, so a window's edge is a cut the page does not make and the move would not make the
  window the page's (trap 216: an exactness proved on one converter is not inherited).
- **What it draws is §3's difference, not this route's.** A stroke under one level of a pixel,
  drawn aliased, marks only the pixels whose centres it passes within a few thousandths of — the
  probe below drew 5 for a 0.002-pixel diagonal and curve — and on a page past the library's range
  those are aliased for the reason §3 measures, whichever vertices the stroker is handed.

## 3. The bound, measured: the library's range is the surface's

A mark the library draws itself — every mark under `carries_coverage_as_alpha`'s `false`, whatever
its converter — has its anti-aliasing withdrawn by `scan::expressible` where its coordinates pass
the library's 8191-pixel range, and that question is asked of the **surface** (ADR 2058; ADR 2133
moved only `crate::area`'s range to the page's grid). A probe in an export of this round: a knockout
group over a page 9000 pixels across, a `0 w` diagonal and curve in it, drawn whole and as a 181 × 88
window inside the last tile. The page, past the range, draws the stroke aliased (323 pixels changed,
every one to the paint); the window, inside it, anti-aliases it; **the window parts from the page at
620 pixels**, and at 0 without the stroke. Outside the knockout group the same window is the page's
(0 pixels). So the bound is: on a page past 8191 pixels, a window or a strip of it draws a
knockout element or an aliased-mode mark anti-aliased where the page draws it aliased. Asking the
library's range of the page's grid would make the window the page's, aliased; drawing such marks
through `crate::area` instead would make both anti-aliased. Either is a pixel change of its own, for
a later round, measured first: the corpus's census of knockout elements on pages past the range is
the first number it owes.
