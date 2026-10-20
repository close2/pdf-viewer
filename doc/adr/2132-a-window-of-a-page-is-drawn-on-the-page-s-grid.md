# 2132 — A window of a page is drawn on the page's grid

Session 1648, slot 2 of batch one hundred and one, a pixels round on ADR 2116's last section, under
the project owner's switch to code improvements and bug fixes of 2026-10-14. Status: **accepted**
and **built** in `render-cpu`, with one line at each of the three callers that draw a window.
Clause: ISO 32000-2 §10.7.4 (the row stays `departed`, ADRs 1060, 1360, 2058; its note gains a
sentence). Context: ADRs 0219, 1930, 2058, 2100, 2106, 2116, 2133; traps 146, 213, 216, 222, 225.
Code: `render_cpu::CpuRasterizer::rasterize_window`, `render_cpu::Shown`, `surface::{Surface,
ToDevice}`, `viewer_host::oversize::Window::shown`; the callers `viewer_host::drawing::draw_window`,
`viewer_confined::worker::draw_windows`, `viewer_ffi::session`'s window. Test:
`render-cpu/tests/window_on_the_page_grid.rs` (four, new).

## 1. What was measured, and what it was

ADR 2116 left a window of a page past `tiny-skia`'s 8191-pixel range "one level from the page drawn
whole": a window was drawn as a target of its own, `Window::target_of`, the page's transform with
the window's corner folded in. Measured first with `window_against_whole` on HEAD, one exploratory
run each, 1200 × 900 viewport, four threads:

| witness | window | differing | worst | under rectangles | under strokes | under other fills |
|---|---|---|---|---|---|---|
| the owner's, 1.25¹⁰ of the fit (11 176 × 7 898) | 1 800 × 1 350 at (4 688, 3 274) | 5 478 | 2 | 521 | 4 043 | 914 |
| `issue12810.pdf`, 4× (6 913 × 10 368) | 1 801 × 1 350 at (2 556, 4 509) | 811 | 1 | 187 | 567 | 49 |

Isolated on the full-size target by the smallest mark, each drawing's matrix printed (traps 213,
222), they were two things, not one:

1. **The window chose its rectangles' converter from its own extent.** A window is a surface under
   8191 pixels, so its rectangles took the library's 8.8 converter where the page, past the range,
   took the closed form (ADRs 2058, 2116). That is trap 146's shape a third time, and it is a choice
   this backend made, not arithmetic.
2. **Every other mark was the same real numbers rounded at another magnitude.** On `issue12810.pdf`
   the two isolated marks are thin strokes measured by `crate::area` on both drawings, under
   `ty` 5 891.76 on the page and 1 382.7598 in the window — the page's value less 4 509 to the bit,
   so the matrix was not what moved. What moved was every sum computed from it (ADR 2133).

## 2. What the clause says, and so which it is

§10.7.4: "Pixel boundaries always fall on integer coordinates in device space", a pixel is
identified by "the location of its corner with minimum horizontal and vertical coordinates", and a
shape's "coordinates are mapped into device space but not rounded to device pixel boundaries". A
window's corner is a whole number of the page's pixels, so every pixel of the window *is* a pixel of
the page, and the integral over its square is the page's. The clause gives no permission for an
answer that depends on where a surface begins: the brief's first possibility — that the remaining
level is the half-open square's own allowance — does not hold, and the difference is a departure
of this backend's, to fix where the measurement points.

## 3. Decision

**A window is drawn on the page's own grid.** `CpuRasterizer::rasterize_window(list, page, shown)`
takes the page's target and the window as a rectangle of it (`Shown`), and `Surface` carries both:
the page's target travels into the draw as it does into a strip (ADR 0219), and `ToDevice` composes
the window's first column last beside its first row. Everything that is a statement about the page
is now the page's: `Scratch::for_page` is handed the page's extent, so a window of a page past 8191
writes its rectangles by the closed form as the page does (ADR 2116); the mask cache's
"covers the surface" asks the page's columns; a thin dot's closed form (ADR 1829) and an
unsmoothed image's centres (ADR 2100) are stated on the page's grid and moved by the corner in
`f64`. Everything that is a statement about the window — which commands reach it, its pixmap,
§14.11.2.1's crop and the medium, the strip planner's rows, a glimpse's rows — is asked of
`Shown::target`, the window's own target, as before.

`Rasterizer::rasterize` is `rasterize_window` of the whole page (`Surface::whole`), so a page draws
through the same lines and nothing about it changes. The three callers that drew a window through
`window.target_of` now call `rasterize_window`; `target_of` stays for what asks only which
commands reach a window (`DisplayList::reaching`, `share_reaching`). A window whose far edge passes
`MAX_EXTENT` is refused as a page past it is, one check for both (`within_extent`).

## 4. Measured

Step by step on exports of HEAD plus this round (ADR 1890), the same runs as §1:

| | the owner's witness | `issue12810.pdf` at 4× |
|---|---|---|
| HEAD | 5 478 differing, worst 2 (521 under rectangles) | 811, worst 1 (187) |
| this ADR alone | 4 084, worst 2 (12 under rectangles) | 624, worst 1 (0) |
| with ADR 2133 | **0** | **0** |

The rectangles went with the converter; the rest is ADR 2133's.

## 5. What this does not do

- `Window::target_of` and the device lanes are unchanged; a device window is the device's.
- A window drawn by a caller that still hands `Rasterizer::rasterize` a shifted target draws as
  before; the tree has no such caller left for a window.
