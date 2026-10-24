# 2314 — A toolkit window's notch is two thirds of its device frame in the glyph atlas, and half of what the reader waits for

Status: accepted — a measurement and its attribution; no product change. Session 1740, 2026-10-24.

## Context

ADR 2266 measured `quorra-qt`'s Control+wheel chain on the owner's display and found the
re-raster per notch dominant, its processor half the larger, at loads of 30 to 44. Batch one
hundred and thirteen's brief asked for one notch split into encode, upload, execute, read back and
present, headless and in one loop run on the owner's adapter, before the owner's zoom without a
re-encode (ADR 0368, `doc/todo/75`) is priced (ADR 2315).

## What a notch on the device lane is

**The toolkit windows draw a notch on `Coverage::Cpu`.** Their read-back lane
(`viewer_device::renderer::read_back`) takes `lane::coverage_for`'s launch rule for every request,
which is `Cpu` below a magnification of 10 (ADR 2070 states the rule per page), where `quorra`'s
window takes `lane::lane_for`, which gives a moved view `Coverage::Compute` (ADR 0700). Under `Cpu`
the glyph lane keys each atlas tile by the composed linear part (raster's `atlas::GlyphKey`), so
every notch rasterises every glyph of the page again. ISO 32000-2's page 20 is 2 659 commands, 2 580
of them glyphs and 79 rectangles, and opens in the OneColumn layout (§7.7.2), so the first notch
from 100 % draws pages 20 and 21; from the fourth notch on, the page is past `viewer_host::
oversize`'s bound and the processor draws the window, so three notches of eight reach the device.

## The figures

**Headless**, the agent's own adapter access (Radeon 890M, RADV), one `--clock` hold at a load
average of 9.7 to 7.8 on 24 threads, `render-raster`'s `zoom_frame` at scale 1.6 with the sequence
`1,1.25,1.5625,1.953125`, minimum of five rounds, two passes; ms, the device frame of one page:

| notch | target | device | encode | transfer | execute | read back | elsewhere |
|---|---|---|---|---|---|---|---|
| 1 | 1191 × 1684 | 9.0, 9.8 | 6.3, 7.1 | 0.3 | 0.3, 0.4 | 1.4, 1.6 | 0.7, 0.5 |
| 2 | 1489 × 2105 | 11.6, 12.3 | 8.3, 7.6 | 0.1 | 0.2, 0.4 | 2.3, 3.5 | 0.6 |
| 3 | 1861 × 2631 | 13.0, 15.4 | 8.7, 9.8 | 0.1, 0.3 | 0.3, 0.6 | 3.3, 4.2 | 0.6 |

The encode is 62 to 72 % of the device frame. `ZOOM_FRAME_ENCODE_PHASES=1` (three rounds, shares
only) divides it: the glyphs' rasterisation into the atlas (`geometry`) 4.6 to 7.8 ms, 68 to 72 % of
the encode, `staging` 0.5 to 0.8 and `recording` 1.6 to 2.2.

**On the owner's adapter**, one loop run in the same hold: Wayland, `eDP-1` 2880 × 1800 at 1.6 and
120 Hz, the window's frame 1000 × 1101 logical, activated by its pid and confirmed the active
window before its notches, no capture; `PDFVIEWER_ZOOM_DRIVE=notches=8,every=400,after=3000` from
`#page=20&zoom=100`, three runs at loads of 6.7 to 5.0. Each notch sums the frames trace's
`phases on the graphics device` lines, its `bytes into a QImage` copies and its `paint-end` lines
from the wheel's own probe timestamp; `settled` is the first paint whose every image is unscaled at
the new size. ms, three runs:

| notch | pages | encode | read back | scene | QImage copies | paints | felt | settled |
|---|---|---|---|---|---|---|---|---|
| 1 | 20, 21 | 12.5, 12.3, 11.2 | 4.4, 3.0, 3.7 | 2.5, 1.6, 0.9 | 3.8, 3.6, 3.9 | 2.7, 3.3, 2.9 | 5.8, 6.8, 6.9 | 25.5, 23.2, 21.6 |
| 2 | 20 | 9.2, 9.7, 13.0 | 2.5, 2.7, 3.5 | 1.1, 0.8, 2.5 | 2.0, 2.3, 1.3 | 2.6, 2.5, 3.9 | 3.6, 3.7, 3.1 | 18.0, 17.7, 23.7 |
| 3 | 20 | 12.9, 11.5, 15.2 | 4.1, 3.9, 4.7 | 1.2, 0.7, 0.6 | 3.1, 3.4, 3.7 | 2.5, 2.5, 4.7 | 3.5, 3.7, 4.8 | 22.5, 20.3, 26.4 |

Transfer and execute are 0.1 to 0.5 ms a page throughout. The same drive under an Xvfb of the
round's own at `QT_SCALE_FACTOR=1.6`, in the hold at loads of 9.2 to 7.8, settled at 18.8 to 25.7 ms
with encodes of 10.2 to 14.7 ms a notch: the display is not the term.

## Decision

1. **The attribution.** The encode is 49 to 58 % of the settled notch on the owner's display and 58
   to 70 % of its device frame; within it, the glyph atlas's rasterisation at the new linear part is
   two thirds or more. The reader meets the scaled stand-in (ADR 1843) 3.1 to 6.9 ms after the
   wheel, inside one 120 Hz frame on every device-lane notch measured.
2. **No product change here.** What would change the settled figure is priced in ADR 2315.

## Costs

Single samples on the owner's display, a minimum of five headless; a quiet machine reads lower.
