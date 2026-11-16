# 2884 — A scene keeps the marks the splits stated, and another view asks their answers again

Session 2034, a ledger round (`doc/todo/76` row 8). Status: implemented. Rows: §8.5.3.2, §10.7.4.
Context: ADRs 0701, 0702, 0705, 1857, 1875, 1877; raster's ADRs 0085, 0086, 0097.
Code: `crates/render-raster/src/stroke.rs` (`keep_marks`, `KeptMark`, `StatedMarks`),
`present.rs` (`Retained::marks`), `scene.rs` (`Encoder::keep_mark`), `pdf-render`'s
`stated_point_mark`. Tests: `render-raster/tests/marks_across_views.rs`. Benchmark:
`render-raster/examples/dotted_zoom.rs`.

## Context

ADR 0705 narrowed the stroke encode's view rule to "marks made": a stroke whose §8.5.3.2 splits
made a dot or a zero-length dash's cap consumed the view, so every zoom step rebuilt the scene of
any page holding one, and deferred "the disc per placement in quorra's encode, the §10.7.4
point-mark substitution mirrored" until a corpus page showed dots on its zoom path. ADR 2800
(A800) lifted that trigger; `doc/todo/76` row 8 is this. ADR 1877 had meanwhile kept the lone
round dot as a raster line, re-asking at each view whether it is still one.

## The premise, checked

Unbuilt, and effort only: no answer in `doc/questions/A*` cites ADR 0705, and the encode's
`consume_view` for marks stood. **The brief's construction did not survive the reading.**
Mirroring `pdf_render::point_mark` into raster's line shader would reproduce the oracle's
substitution for a lone dot, but not the oracle's decision for a dot inside a path: `render-cpu`
takes such dots by `pdf_render::thin_dots`, which declines the whole path's dots when any two meet
one pixel (ADR 1875), so the oracle's answer for one mark depends on its neighbours at this view.
A per-placement shader cannot ask that; it would draw a second answer beside the shared one,
which is the two-decisions defect every comment in `stroke.rs` cites trap 2 against.

## Decision

The marks stay `pdf-render`'s construction, and **the scene keeps the answers they were stated
by**. `split_degenerate` and `split_dash_marks` read the view through two values only: the width
`Stroke::device_width` resolves (§8.4.3.2's zero, §10.7.5's adjustment) and the `PointMark`
`stated_point_mark` — now public, the one function both splits already called — decides at it
(§10.7.4). Which subpaths are degenerate is the path's geometry and the dash cuts in path space
(ADR 0705). So a stroke that made marks records `StatedMarks` (its transform, stroke, whether the
paint takes a substitution, the width and the `PointMark`), and `Retained::draws` keeps a
view-free scene at another view only where every record's width is the same bits there and
`stated_point_mark` returns the same answer; the kept scene is then that view's build to the bit.
A lone round dot left to the splits also asks ADR 1877's line question, since a view that draws it
as a line draws something else. **One answer still consumes the view**: a mark §10.7.4 restates
as the device pixel its centre lies in ("let i = floor( x ) and j = floor( y )") is this grid's
pixel, which no other placement names.

What is kept: marks a device pixel across or more on a zoom, and restated marks on a scroll at one
scale, whose substitute width is the same pixel at every translation. What rebuilds: a step that
crosses §10.7.4's boundary, a zero width (`device_width` is a pixel, which a zoom changes), and an
adjusted width under half a pixel.

## Measured

`examples/dotted_zoom`, release, five rounds, minimum per step, load 19, on the software adapter
(lavapipe): the run was unpinned, and this VM had no render node for the RX 580 that day
(`/dev/dri` held `card0` alone, and a run pinned to `radeon_icd.json` found no adapter), so wgpu
took the processor's Vulkan. The A/B is one binary on one adapter, so the ratio stands; the device's
own figure is owed when the card is back: 150 dotted leaders, 27 750 round marks, scales 1 → 1.25 → 1.5 → 2 → 1.25 → 1. Kept:
every step keeps, 10.7–19.8 ms a frame, scene 0.01 ms. The same page with one dot restated as its
pixel — every step rebuilds, the route all marks took before — 31.7–42.1 ms, scene 7.5–11.9 ms.
The first frame is the same in both (70–75 ms).

## Held by

`marks_across_views.rs` on the adapter (lavapipe, for the reason above), byte for byte against a device that built at each view:
a page of round and square marks 2.5 units wide kept across six views; 0.6-unit marks rebuilt on
the step from scale 2 to 1 and kept on a scroll at scale 1; 0.02-unit marks rebuilt on a scroll.
Planted back: a record that always holds fails the second; a pixel-restated mark kept fails the
third; marks that always consume fail the first two.
