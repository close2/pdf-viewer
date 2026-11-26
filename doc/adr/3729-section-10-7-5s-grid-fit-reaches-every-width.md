# 3729 — §10.7.5's grid fit reaches every width

Status: implemented. Session 2257, fixing audit 2248's B05. Context: ISO 32000-2 §10.7.5; ADRs 0848,
2530, 2553; `doc/questions/A714`. Code: `crates/pdf-render/src/grid_fit.rs` (`fitted_width`,
`fitted_centre`), `raster/crates/raster-gpu/src/raster/grid_fit.rs`, `raster-gpu/src/encode/{stroke.rs,
line.rs}`, `raster-gpu/src/shaders/line.wgsl`, `crates/render-gpu/src/scene.rs`. Tests:
`crates/pdf-model/tests/rendering_10_5_10_8_spec_audit.rs::stroke_adjustment_reaches_a_rule_two_pixels_wide_spec_audit`,
`crates/pdf-render/src/grid_fit.rs`, `crates/render-raster/tests/stroke_adjustment.rs`.

## 1. The finding

> When stroke adjustment is enabled, the line width and the coordinates of a stroke shall
> automatically be adjusted as necessary to produce lines of uniform thickness. The thickness shall be
> as near as possible to the requested line width -no more than half a pixel different.

ADR 2530 fitted only strokes under 1.5 device pixels (`FIT_BELOW`), to one whole pixel. The sentence
has no width in it: it binds every stroke `/SA` enables. Under ADR 2530's own reading of *uniform
thickness* — the clause's first paragraph, "[a] line of a given width can intersect with different
numbers of device pixels, depending on where it is positioned" — a two-point rule at y 50.25 covered a
quarter row, a whole row and three quarters of a row, where the same rule at 50.0 covers two whole rows.

## 2. The rule, stated once

- **The width** is the whole number of device pixels nearest the requested device width, and never
  fewer than one: `max(1, ⌈w − ½⌉)`. That is within half a pixel of every requested width of half a
  pixel or more — the clause's bound — and a width under half a pixel is one pixel by the clause's
  second requirement already. **A tie takes the smaller**: 1.5 and 2.5 are as near to 1 and 2 as to 2
  and 3, the clause prefers neither, and the smaller keeps the old bound's answer at 1.5 and lays down
  less ink than the document asked rather than more. This is a choice, and it is written as one.
- **The centre** across a segment moves to the nearest place where both edges of a stroke that wide lie
  on pixel boundaries: `⌊v − n/2 + ½⌋ + n/2` — a pixel centre for an odd width (`⌊v⌋ + ½` at one, the
  old rule exactly) and a pixel boundary for an even one. The places are a whole pixel apart, so the
  move is at most half a pixel, as before.
- **Everything else is ADR 2530's**: segments along the device axes only, a transform stretching both
  axes alike, a free end left where the document ended it, a curve or diagonal unfitted (ADR 0848).

`pdf_render::fitted_width` and `fitted_centre` state the two formulas; `raster`'s `grid_fit` restates
them statement for statement (the library depends on no PDF crate), and `line.wgsl`'s vertex stage
uses the same operations in the same order, so the host's `fit_on_device` and the shader agree to the
ulp ADR 2022 allows. `raster`'s fit now returns the fitted width beside the polylines, and a fitted
round dot is that width across.

## 3. render-gpu

`render-gpu` (vello, the cross-backend comparison) never fitted: ADR 2553's "every backend" was
`render-cpu` and `raster`'s lanes. It now calls `pdf_render::grid_fit` at `encode_stroke`'s entry, as
`render-cpu` does at its stroker's, so the comparison backend draws the same clause.

## 4. What it costs

Wider rules under `/SA true` move by up to half a pixel and change width by up to half a pixel; on a
page that stacks a wide rule against a fill, a seam can open or close where ADR 2530 measured it for
thin rules only. The golden pages that moved are named in the round's record.
