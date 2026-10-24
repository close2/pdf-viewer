# 2315 — The zoom that re-encodes nothing is `quorra`'s already, and a toolkit notch's floor without its encode is past a frame

Status: rejected — no re-used encode is built for the toolkit windows' notch, because its ceiling
leaves eight of nine notches above one 120 Hz frame. Session 1740, 2026-10-24.

## Context

The owner asked on 2026-08-25 for a zoom that happens on the device without re-encoding the page
(ADR 0368; `doc/todo/75`'s priority). Batch one hundred and thirteen's brief named the lever — the
encoded scene in page space, the transform applied at execute, so a zoom step re-uses the encode and
changes only the transform and the target — and asked for it priced at its ceiling first (habit
164) and built only if a notch then gets under 8.33 ms on the owner's figures. ADR 2314 is the split
this prices against.

## 1. Whether the encode is zoom-invariant today

**Under `Coverage::Compute` it is, and has been since raster's ADR 0087.** The first encode writes
each command's viewport-free half down as a record (the outline, its control box and §10.7.4
collapse table, the command's transform, paint, rule, style and clip), and a later frame at another
view replays the records, computing every device-space answer — the composed transform, the cull,
the seat, the instance bytes — by the walk's own arithmetic. On page 20 every step after the first
frame says `record-replayed`, its encode 0.3 to 0.5 ms; the replayed 1191 × 1684 frame's digest is
`fc80bd2dd3ca42d1`, and a fresh walk of that view alone reads the same. A scene with a child layer,
a soft mask, a residue clip, an atlas or a winding tile re-walks instead.

**Under `Coverage::Cpu`, the lane a toolkit notch takes (ADR 2314), it is not**, and what is not is
the larger part of it:

- **the glyph lane**: a tile is keyed by the composed linear part and rasterised at the placement's
  fractional phase quantised on the device grid (raster's `atlas::GlyphPlacement`, ADR 0498's
  carry) — the `geometry` share, 68 to 72 % of the encode on page 20;
- **the lane itself**: a fill becomes a glyph where its device size fits an atlas tile and a path
  where it does not, so a step moves marks between lanes;
- **the thin-mark condition** (raster's ADR 0070, amended by ADR 2296): the width below which the
  device lane cannot promise §10.7.4's "any pixel whose half-open square region intersects the
  shape" is a device width, so a stroke's lane turns with the magnification;
- **the rectangle lane**: the device rectangle and its clip's intersection are computed in device
  space (raster's ADR 0007).

**What a re-used `Cpu` encode would draw wrong** is every glyph: the old view's tiles placed under
the new transform are the old grid's coverage resampled at 1.25 times, not §10.7.4's scan
conversion on the new grid — the picture the scaled stand-in (ADR 1843) already paints within a
frame of the wheel. Keeping the glyphs right means rasterising them again, which is the share above.

## 2. The ceiling, on the owner's figures

The shares a re-used encode cannot remove — the scene walk, transfer, execute, read back, the
device call's remainder, the copies into a `QImage` and the paints — are the settled notch less its
encode, in ADR 2314's three runs: 13.0, 8.8, 9.6; 10.9, 8.0, 8.8; 10.4, 10.7, 11.2 ms. **With the
encode at zero, one notch of nine is under 8.33 ms**, and that zero draws the stand-in's picture.
With the glyphs re-quantised per view, what is left to re-use is `staging` and `recording`, 2.1 to
3.1 ms headless, and no notch gets under.

The route that already exists is the read-back lane asking `lane::lane_for` instead of
`lane::coverage_for`, so that a moved view takes `Compute` and its replay. Headless, in the same
hold, its device frame of page 20 read 4.3 to 5.2, 6.7 to 7.3 and 9.1 to 9.4 ms at the three notches
against `Cpu`'s 9.0 to 9.8, 11.6 to 12.3 and 13.0 to 15.4. Put in place of the owner's device
frames, with the time outside them unchanged (3.1 to 4.7 ms a notch) and page 21 priced as page 20,
it gives 12.6 to 14.9, 10.0 to 11.4 and 12.2 to 14.1 ms: no notch under. It would also change what
the windows show: against `Cpu`'s frame at the first notch, 7.30 % of the pixels differ, the worst
by 14 of 255 (raster's ADR 0082 relaxes the lanes to a close best effort, not to equality). So it is
not built either.

## 3. Decision

1. **No re-used encode for the toolkit notch, and the read-back lane keeps `coverage_for`.** Neither
   reaches the bar on the owner's figures.
2. **What blocks a toolkit notch from a frame is the toolkit's own floor, not the encode**: the read
   back of the whole page (2.5 to 4.7 ms a notch), its copy into a `QImage` (1.3 to 3.9 ms), the
   paints (2.5 to 4.7 ms), and under the OneColumn layout the second page at the first notch. A path
   that presents from the device, which neither toolkit hands a program (ADRs 0244, 0246), or a
   device lane that draws and reads back the viewport's window rather than the page, is the next
   lever to price; this ADR builds neither.
3. **The owner's zoom is `quorra`'s today.** Its window takes `Compute` on a moved view: headless,
   a 990 × 1595 window target of page 20 through eight notches read a device frame of 4.5 to 8.5 ms
   with an encode of 0.1 to 0.4, of which a read back of 1.4 to 2.0 ms that a presented window does
   not pay; what a step uploads is the records and quads of the fills that reach the window, at most
   374 048 bytes, and the device rasterises them at the new view.

## Costs

The ceiling is arithmetic on single samples at loads of 5.0 to 6.7; a quiet session would move every
figure down and the floor with them. Reopened by a toolkit path without the read back or the copy.
