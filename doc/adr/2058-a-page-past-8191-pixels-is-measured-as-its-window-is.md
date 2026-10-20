# 2058 — A page past 8191 pixels is measured as its window is

Session 1611, slot 5 of batch ninety-six, a clause round on ADR 1930 §4, under the project owner's
switch to code improvements and bug fixes of 2026-10-14. Status: **accepted** and **built** in
`render-cpu`. Clause: ISO 32000-2 §10.7.4; the row stays `departed` (ADRs 1060, 1360), its note
gains a sentence. Context: ADRs 0219, 0269, 0476, 1082, 1817, 1837, 1930. Code: `render_cpu::scan`
(`fill_as`, `fill_rectangles`, `intersected`, `mask_fill`, `RECTANGLE_SURFACE_LIMIT`,
`rectangle_converter_reaches`), comments in `area.rs` and `strips.rs`. Tests:
`render-cpu/tests/page_past_the_tile.rs` (three). Instrument: `viewer-host`'s
`examples/window_against_whole`, which now sorts the differing pixels by their distance to the
window's edge and by the class of the smallest mark holding each, and narrows the most different
pixel to its marks where `WINDOW_AGAINST_WHOLE_ISOLATE` is set.

## 1. What was found

ADR 1930 §4 left a difference unexplained: at 1.25¹⁰ of the fit on the owner's witness, 6 889 of a
window's pixels differed from the same pixels of the page drawn whole, 914 by more than 8 levels and
at most 56. §10.7.4 makes a pixel the half-open square `[i, i+1) × [j, j+1)` on integer device
coordinates and says a shape's "coordinates are mapped into device space but not rounded to device
pixel boundaries", so the same marks at the same scale owe the same pixels wherever the grid is the
same, and a window's grid is the page's: its corner is a whole number of pixels.

**The brief's hypothesis did not hold.** The window's origin is not off the page's grid —
`Window::target_of` subtracts whole pixels — and the difference was neither ADR 1817's seam nor a
strip: the whole page has one digest at one, four and twenty-four threads. Below 8191 pixels the
two drawings agree to at most 2 levels at every magnification measured (1.25², 1.25⁵, 1.25⁸): 83 to
1 736 pixels, ADR 0219's arithmetic at a shifted origin, which the window's corner is.

**What it was is the surface's size.** At 1.25¹⁰ the page is 11 176 pixels wide, and `tiny-skia`'s
fixed point stops at 8191 (`scan::SUPERSAMPLED_LIMIT`, and the library's `DrawTiler`). The window,
nearer the origin, is under it; the page is not. The instrument narrowed the most different pixel
to one command: a glyph whose bar is an axis-aligned rectangle, 0 on the page and 58 in its window,
drawn alone; on a page target cropped under 8191 it was 58 too. Two consequences of the range
reached the page and spared the window:

1. **Every rectangle on a surface past 8191 lost its closed form.** `PixmapMut::fill_rect` takes the
   library's rectangle converter only where its `DrawTiler` is not required; on a larger surface it
   builds a path and hands it to the supersampled converter, which rounds an axis-aligned edge to a
   quarter of a pixel — ADR 0476's defect, back on every large page. This was the 58.
2. **Every mark whose extent passes 8191 was drawn without anti-aliasing**, however small: the
   range rule (ADR 0269) withdrew anti-aliasing from the mark before `render_cpu::area` was asked,
   although that converter states every coordinate against the mark's own region in `f32` and has
   no such limit. On this page that is everything right of x = 8191, about a quarter of it; the
   window measured did not reach there, and a window that does would differ at every edge.

`render-cpu/tests/page_past_the_tile.rs` is both halves with no document in the way, and fails on
HEAD at all three tests: a triangle and a stroke past 8191 drawn 0 or 255 at every boundary pixel
against the same marks moved into range by a whole 8000 pixels, and a rectangle's edge 64 where its
area is 58.

## 2. Decision

1. **The range withdraws the library's anti-aliasing only, and `area` has a range of its own.**
   `fill_as` and `fill_rectangles` hand `intersected` the anti-aliasing the mark asked for wherever
   its device extent lies inside `MEASURED_LIMIT` (32 767), and the library's fallback the withdrawn
   one; `mask_fill` asks `area` under the same bound and gives the library its range. Below 2^15 an
   `f32` coordinate is stated to 2^-9 of a pixel, under the integral's flattening tolerance; past it,
   or where the transform states no bounds, a mark is drawn by the library aliased exactly as
   before, so ADR 0269's abort stays unreachable and its two witnesses draw as they did (§4).
2. **A rectangle on a surface past `RECTANGLE_SURFACE_LIMIT` (8191) is written by its closed form
   into the coverage buffer** (`intersected`'s `exactly`), which is the construction a clipped
   rectangle already takes (ADR 0476), and the buffer is drawn through as whole pixels.
3. **What is left is stated, not removed**: a page and its window agree to ADR 0219's arithmetic at
   a shifted origin — the window's marks are mapped at another magnitude — and, for a rectangle, to
   the one level between the library's 8.8 fixed point and this backend's `f32` statement of one
   product. Removing either would draw every window on the page's own surface, which is the cost
   ADR 0219 already declined for strips.
4. **No `§10.7.4` permission is claimed.** The clause permits nothing here; both drawings were
   meant to be one, and now are to the rounding above.

## 3. Measured

One `--clock` hold (`scratchpad/r1611/hold.sh`, load 1.2–2.3), HEAD plus the instrument against
that plus this change, each its own export and target, at 1200 × 900, one round per magnification:

| threads | 1.25², 1.25⁵, 1.25⁸ | 1.25¹⁰ differing, worst | whole page at 1.25¹⁰ | window at 1.25¹⁰ |
|---|---|---|---|---|
| 4 | every digest HEAD's | 6 853, 58 → **6 085, 2** | 19 414 → 21 740 ms | 220 → 261 ms, HEAD's digest |
| 24 | every digest HEAD's | 6 843, 58 → **6 075, 2** | 19 190 → 20 921 ms | 179 → 192 ms, HEAD's digest |

The whole page at 1.25¹⁰ changes its digest (the same at both thread counts) and nothing below 8191
pixels does. Its cost, one sample each, is 1.7–2.3 s more of about 20: the quarter of the page past
the range is measured by `area` where it was filled aliased. That page is drawn in one strip at any
thread count because its four frame strokes reach past the range and ADR 1837's planner forbids
their rows, which `strips.rs` still does — a lever for a later round, priced here at nothing.

**The corpus gate draws no window**, so what it can say is the page half. Its six arms against
HEAD's (`/home/AI/arms-1607/`), by page: 0 of 968, 968, 968, 965, 965 and 965 frame digests differ —
the frames the arms digest are the raster backend's lanes, not `render-cpu`'s — and the mean error
against the oracle, which is `render-cpu`, moves on four pages at 4× alone, each down:
`issue14497.pdf` 0.0509 → 0.0006 (cpu), 0.0505 → 0.0001 (compute), `issue19207.pdf` 0.0387 →
0.0120, `issue12810.pdf` 0.0113 → 0.0108, `issue16287.pdf` 0.0003 → 0.0002. Those pages drawn at
4× before and after and looked at (trap 1): `issue19207.pdf`'s heading, which crosses x = 8191, has
stepped edges past that column before and anti-aliased ones after, the same letters; the page
frame of `issue14497.pdf` (13 480 pixels wide) is a two-column aliased line before and a line with
its partial column after. The device lanes were never under the library's range, which is evidence
about the reading and no more (principle 5). `raster_golden`: held 974, moved 0.

## 4. What the first build missed, and why the bound is written

The first build handed `area` every mark the range had withdrawn anti-aliasing from, on the
reading that the range is only the library's fixed point. It is, but `expressible`'s `false` also
covered ADR 0269's hostile geometry — a fill to 10²⁵ device units on a 368 × 542 page — and `area`'s
`f32` integral of such an edge resolves nothing where it crosses the raster. The batch's
`fixed_documents` gate caught it: `4605705.pdf`, one of ADR 0269's two witnesses, drew 98.824 of
ink against the 145.204–147.204 its round recorded. Neither this ADR's tests nor `scan.rs`'s
witness test saw it, because the first moved marks by whole pixels near 8191 and the second asked
only that *something* was drawn. `page_past_the_tile.rs`'s fourth test pins the shape — a spike
to 10⁷ and to 10²⁵ draws byte for byte as the library's aliased converter draws it — and was red
on the first build (108 pixels at 10⁷). The bound costs ADR 2058 nothing it set out to do: a page
would have to be drawn wider or taller than 32 767 pixels for a mark on it to pass the bound, and
the witness at 1.25¹⁰ is 11 176.

## 5. What this does not do

- The window path, `oversize.rs` and the device's lanes are unchanged.
- The planner's range condition stays as it was, conservative now that `area` measures past it.
