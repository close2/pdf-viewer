# 1829 — A straight stroke inside two by two pixels is painted from its closed-form area

Session 1496. Status: **accepted** and **built**. ADR 1817's first increment, `doc/todo/69` §1, for
the project owner's performance target of 2026-10-10. Context: ADRs 0226, 0268, 0355, 0418, 0476,
0945, 1082, 1102, 1374, 1742, 1758, 1817, 1819; traps 1, 2, 13, 56, 80, 134. Code:
`crates/pdf-render/src/thin_segment.rs` (`thin_segment`, `ThinSegment`, `disc_in_triangle`),
`crates/render-cpu/src/thin_segment.rs` (`draw`), `render-cpu/src/lib.rs`'s `draw_stroke`. Tests:
`crates/render-cpu/tests/thin_segment_ladder.rs`, the module's seven unit tests. Clauses: §8.4.3.2,
§8.4.3.3, §10.7.4, §11.3.6. No ledger row moves.

## 1. The population, and where it is decided

`pdf_render::thin_segment` admits a stroke that is one `m` and one `l` of non-zero length, with no
dash, a solid paint and `Normal` blend, under a transform that keeps the axes, whose ink box meets
at most two pixel columns and two pixel rows. It decides in `pdf-render`, so that every backend
answers the same marks (trap 2). Round 1499's device primitive takes its yes or no.

**The segment may run in any direction.** The first build admitted only segments along a device
axis, since a band along an axis has a separable area. On the witness at the fit that admitted
3 699 730 of 4 400 060 strokes. The 667 904 diagonal hatching dots that stayed on the general path
were still 13.3 G of the drawing's 34.3 G instructions under callgrind. ADR 1817's own population was
conditioned on the transform keeping the axes, not on the segment. Admitting any direction admits
4 367 627. What it still declines on the witness: 28 684 zero-length dots (§8.5.3.2,
`split_degenerate`'s), 2 588 longer axis rules, and 1 128 multi-segment paths.

## 2. The area

A transform that keeps the axes maps the half-width's disc to an axis-aligned ellipse, with
semi-axes `half_x` and `half_y`. Dividing the device axes by those semi-axes, composed with the
transform, is a similarity of the path's space. So in that frame the shape is §8.4.3.2's set again:
the points within one of the divided segment, with Table 53's ends square to it. There, a pixel is a
rectangle; the band and a projecting end are a convex quadrilateral, clipped to it and measured by
the shoelace formula. A round end is the unit disc beyond the line through its endpoint. Its area is
summed edge by edge over the clipped rectangle: a triangle where the edge is inside the circle, a
sector where it is outside. A block's pixels partition the shape. One pixel holds `area()`, with no
geometry. Two need one area. Four need three: a corner, and the two halves it lies in.

## 3. The composition

`render_cpu::thin_segment::draw` rounds each area to the nearest level. The level is lifted to one
in every pixel only for a mark none of whose pixels reaches a level (ADR 1374's rule, the
rectangle's). It then meets the clip and soft mask as `scan::intersected` does: `min` with `C · S`,
`scan::scaled` by `S`. It lays the paint on by §11.3.6's `(1 − αs) · cb + cs`, in the order of
operations `tiny-skia`'s high-precision pipeline uses for a solid colour through a mask: the mask
over 255, the backdrop as a level times the reciprocal of 255, unfused, rounded ties to even. So a
pixel both are handed at one coverage is one byte. A clip buffer of another size is declined to the
general path, as `intersected` declines it.

## 4. Evidence

- **The ladder**, `thin_segment_ladder.rs`: every pixel of more than 9 000 axis marks, each one of
  81 placements, at seven widths from 0.047 to 1 px, four lengths, three caps and both axes. Its
  expected areas are integrated from the set's chord lengths at 20 000 steps. The turned marks
  (more than 2 000, four directions, three pens including (½, ¼)) are held against a 4 096-gon of
  the set, clipped exactly. An anisotropic rung, a mark on the raster's edge, the lifted floor, and a control:
  one segment and the same band as two collinear segments under five fractional clips are within
  one level. Calibrated: a 2 % deficit planted in one round end, and a sector divided by 2.02, each
  fail the ladder; both are reverted.
- **`raster_golden`**: three of 967 first pages move, all "raster only". `issue12295.pdf`: 6 746
  pixels, 6 736 of them darker, up to 10 levels, ink +0.58 %. These are ADR 0945's anisotropic
  sub-pixel rules, which the widened band with its width in the alpha had drawn light.
  `bug1978317.pdf`: 1 442 pixels at one level, both ways. `issue12810.pdf`: 1 785 pixels at one or
  two levels. The division test holds: 173 pages move at some division, at most 946 pixels and
  3 levels, and the 11 named stay within their ceilings. Each page was looked at (trap 1).
- **The witness**, `render_at` HEAD against this change: at the fit, 48 761 pixels differ (3.4 %).
  43 615 differ by one level, the most by 12, and the ink rises by 0.016 %. At four times the fit,
  748 422 pixels differ, 717 054 of them by one level, the most by 4.

## 5. The hypothesis, read

It said the areas agree with HEAD's bytes wherever HEAD was exact, and differ only by a round cap's
flattened chord. **It held in kind and not in size.** Per mark the difference is under a level:
HEAD's area of the stroker's flattened outline against the true one, plus rounding at the margin.
But the witness's dots overlap three deep and its runs are hundreds long. Per-mark rounding,
composited mark after mark, reaches 12 levels in a column where a run of 0.07-px dots straddles a
pixel boundary. That is the arithmetic of separate composition (§11.3.6, one mark at a time), not a
defect of either area. Where HEAD restated a sub-pixel rule as a wider band with its width in the
alpha (`issue12295.pdf`), the exact area is darker. That is the direction §10.7.4's "at least as
large as the area of the original shape" asks for.

## 6. What it buys, and what is left

`tile_price`, one clock hold, pinned to the eight fast cores, HEAD export against HEAD plus this:
the fit page on one thread **2 933.1 / 2 835.0 ms → 834.0 / 844.8 ms**. The four-times window, whole
list, 913.1 / 926.1 → 624.6 ms. Four tiles on four threads, 2 383.2 / 2 443.7 → 474.2 / 794.6 ms.
The floor, 137.8 ms, is unchanged.

What is left of the 834 ms is the command loop, not the kernel: in callgrind, `device_bounds` is
9.9 % of the run. Every strip replays every command's extent test (ADR 0137). `drawn_row_by_row`
still forbids a cut at a stroke under `unmeasurable_width`, which this kernel draws pixel by pixel.
Both are the strip plan's, which is slot 5's this batch. A zero-length round dot is a disc, which
this construction measures already. It stays with `split_degenerate` until a round says it should
not.
