# 2902 — A dashed stroke the fit declines is kept only where another view declines it too

Session 2043, a ledger round (raster findings, `findings-queue.md` "From round 2034"). Status:
implemented. Rows: §10.7.5 (stays `implemented`). Context: ADRs 0702, 0705, 1877, 2530, 2553, 2884.
Code: `crates/render-raster/src/stroke.rs` (`fitted_for_dashing`, `UnfittedDashes`,
`KeptMark::Unfitted`). Tests: `render-raster/tests/fitted_dashes_across_views.rs`.

## Context

Round 2034 flagged that `fitted_for_dashing` reads the view through `pdf_render::grid_fit` and
consumed it only when the fit took. §10.7.5 — "When stroke adjustment is enabled, the line width
and the coordinates of a stroke shall automatically be adjusted as necessary to produce lines of
uniform thickness" — is a device question, and ADR 2530's construction answers it with device
measures: a width under `FIT_BELOW` device pixels, every segment on a device axis, and a fit that
would move something. §8.5.3.2 cuts the dash pattern along the path, and the fit moves a corner
along the segments it ends, so the dashes after it fall elsewhere. A decline at one view is
therefore an answer of that view, and a scene kept at a view where the fit takes holds the
unfitted path's dashes where a build there holds the fitted path's.

## The finding, checked

A defect. On the RX 580, a closed rectangle dashed `[3 2] 0` under `/SA`, 0.6 units wide, built at
scale 4 (2.4 device pixels: declined) and kept at scale 1 (fitted) drew 168 of 172 800 bytes unlike
a scene built at scale 1; a one-unit rectangle on pixel centres (already fitted: declined), kept
across a quarter-pixel scroll, the same 168.

## Decision

The decline is kept as a question, as ADR 2884 keeps the marks' answers: `fitted_for_dashing` records
an `UnfittedDashes` (the path, the transform, the stroke) for a dashed stroke under `/SA` whose fit
declines, and `Retained::draws` keeps a view-free scene at another view only where `grid_fit`
declines there too; where it takes, the scene is rebuilt. A stroke without `/SA` is declined by the
fit's first test at every view and records nothing; a fit that takes still consumes the view.
Consuming the view on every decline was the alternative, and it would rebuild every zoom of a page of
wide dashed rules under `/SA` for an answer that is the same at every step.

## Held by

`fitted_dashes_across_views.rs` on the hardware adapter, byte for byte against a device that built at
each view: the zoom that brings the width under the bound rebuilds, a scroll at scale 4 keeps, and a
scroll off the pixel centres rebuilds. Planted back (the record dropped), both tests fail on the bytes.

## ADR 2884's figures, on the device

ADR 2884 measured `examples/dotted_zoom` on lavapipe, the card being absent that day, and owed the
device's figure. On the RX 580 through RADV (`DiscreteGpu` in the lock log), release, five rounds,
minimum per step, load 2.4: kept, 6.8–13.6 ms a step, scene 0.00–0.01 ms; rebuilt, 28.7–30.2 ms,
scene 7.6–10.0 ms; the first frame 36–37 ms in both. The ratio ADR 2884 argued from holds (two to
four times), and `StatedMarks`' comment now carries these figures.
