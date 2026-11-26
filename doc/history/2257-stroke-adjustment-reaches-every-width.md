# 2257 — §10.7.5's grid fit reaches a stroke of any width

Lane `gwg`. ADR 3729. Subject: audit 2248's B05 — `/SA true` fitted only strokes under 1.5 device
pixels (`FIT_BELOW`), while §10.7.5's first sentence binds every stroke the parameter enables.

- **The rule, stated once** in `pdf_render::grid_fit`: `fitted_width` is the nearest whole number of
  device pixels, never fewer than one, a tie taking the smaller (a choice, ADR 3729 §2); and
  `fitted_centre` moves the centre across a segment to the nearest place where both edges lie on
  pixel boundaries, `⌊v − n/2 + ½⌋ + n/2` — `⌊v⌋ + ½` at one pixel, the old rule exactly.
  `FIT_BELOW` is gone. `raster-gpu` restates both statement for statement (`raster::grid_fit` now
  returns the width beside the polylines; a fitted round dot is that width across),
  `fit_on_device` and `line.wgsl`'s vertex stage take the same operations in the same order.
- **`render-gpu` fits too**: ADR 2553's "every backend" had been `render-cpu` and the lanes;
  `encode_stroke` now strokes `grid_fit`'s path. A new cross-backend test,
  `cpu_and_gpu_draw_a_stroke_adjusted_rule_as_whole_rows_at_every_placement`, failed with the fit
  planted out of `render-gpu` (`[179, 179]` for the 0.6 rule) and passes with it.
- The lanes: `render-raster/tests/stroke_adjustment.rs` now compares a 2-, a 2.7-pixel rule and a
  2.5-pixel outline at eight placements in all three lanes on the RX 580; all within 2 levels.
- `fitted_dashes_across_views`'s zoom test rested on the bound ("2.4 pixels, which the fit leaves
  alone"); it now declines at scale 5, where 0.6 units are three whole pixels on pixel centres.
  Planted: a quarter-pixel scroll there fails the test's `kept`.
- Turn pages: `stroke_grid_fit_measure` on `issue14415.pdf` and `bug1743245.pdf` finds no `/SA`
  stroke on the first and 268 on the second, none fitted (its stretches differ by 0.125 %, which the
  fit declines before it reads a width, as before), 0 pixels changed; the work per stroke is the
  same test in the same order, so neither turn can move by this change. No clock walk was run.
- Manifest: B05's entry removed; 2248's `findings.json` B05 `fixed`. Ledger 10.7.5 stays
  `implemented`, its note, code and tests restated; 10.7.4 re-read, `departed`, untouched.

**Gates.** Over `-p pdf-render -p pdf-model -p render-cpu -p render-raster -p render-gpu -p raster-gpu
-p viewer-qt`: fmt 0; clippy `-D warnings --all-targets` 0 (after one `float_cmp` in a new test);
`pdf-sandbox --bins` 0; nextest `--lib --bins --tests` (`xvfb-run -a`) 4713 of 4718, the four
manifest §10.5 entries and the zoom test since restated (`audit-diff.py`: one new, that test,
which passes on the final source with its file); rustdoc of the six crates 0; fuzz not checked
(only `FIT_BELOW` left the public API and nothing in `fuzz/` names it); `raster_golden` 2 moved
(`issue13325_reduced`, `issue21570`), updated through the walk, 0 on its strips check;
`drawn_ahead_in_strips` 0; `cargo test -p conformance` 0 on the final tree.
