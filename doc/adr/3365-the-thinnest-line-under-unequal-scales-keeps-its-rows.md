# 3365 — The thinnest line under unequal scales keeps its rows

Session 2166. Status: accepted. Context: ADR 3333 (the thinnest line under unequal scales is stroked in device
space), ADRs 0139, 1758, 1819 and 1847 (which marks forbid a cut, and the division bound), ADR 1016's companion
test `every_first_page_drawn_in_strips_is_the_page_drawn_whole_within_the_bound` in
`crates/pdf-model/tests/raster_golden.rs`, `crates/render-cpu/src/strips.rs`. Clauses: ISO 32000-2 §8.4.3.2, §10.7.5.

## The failure

Round 2163 found the strips test failing on `bug1743245.pdf`: 14 789 of 264 330 pixels moved, worst 4 levels, most at
2 strips, the first at (33, 0). The page is a ruled sheet: 36 rules across and 27 down the whole page, each `0.5 w`
under `/SA true` and a matrix of scales 0.316 955 and 0.317 350 — 0.12 % apart, past `grid_fit::ALIKE`'s 1/1024 — so
§10.7.5 promotes each to §8.4.3.2's thinnest line and ADR 3333 strokes it as `tiny-skia`'s hairline in device space.

## The cause

Round 2166's scratch dump, built at `7e990758~1` and drawing each first page whole and at 2, 4, 8 and 16 strips,
moves no pixel of the page at any division; built at `7e990758` (round 2158, ADR 3333) it moves the strips test's
numbers. `7e990758` sent these rules to the hairline (`stroke.rs`'s `strokes_in_device_space` branch) and took them
out of `pdf_render::thin_segment`, but `strips.rs`'s `converted_row_by_row` — the planner's question whether a mark
is drawn a row at a time by `area`, so that a cut through it moves nothing (ADR 1819) — was not told. It read the
stroke's `device_width`, one pixel in the path's space, found it measurable, and granted the cut. The hairline is not
drawn by `area`: each strip hands `tiny-skia` the whole line to clip at its own edge, and nothing in the library's
hairline walk promises that a clipped line lays the same coverage on the rows it keeps. Measured, it does not: the
first moved pixel is in row 0, in the strip above the cut.

## The decision

`converted_row_by_row` answers `false` for a stroke `strokes_in_device_space` holds of, as it already did for a width
under one level of a pixel, which reaches the same hairline. The bound is not moved.

The other candidate — handing the hairline the page's own vertices less the strip's corner, as `scan::Place` does
for the library's fill converter — was built and measured, and it is not taken: with it alone the strips test still
fails with the same numbers, and with the predicate alone it passes. It would have been code no measurement asked
for.

## The evidence

- The tracked corpus's strips test (bounded walk, `--profile gates`): passes with the change; with the predicate
  planted back it fails on `bug1743245.pdf` with round 2163's numbers exactly; with the vertex change instead of the
  predicate, the same failure.
- `render-cpu/tests/all/stroke_cuts.rs`'s `the_thinnest_line_under_unequal_scales_keeps_its_rows`: 63 such rules
  (every sixth dashed) on the upper half of a page of short rules; each is asked and keeps its rows, the page is
  still cut, and every division from 2 to 16 draws it byte for byte. Planted back, 12 428 pixels move by up to 9
  levels at 2 strips.

## The cost

A page of such rules down its whole height is drawn in one strip, as it was before ADR 1819 for every stroke. The
page drawn whole is unchanged: the golden draws in one strip and no golden digest moves by this change.
