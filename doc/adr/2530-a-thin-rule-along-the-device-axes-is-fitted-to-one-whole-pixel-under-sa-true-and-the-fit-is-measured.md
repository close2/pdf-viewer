# 2530 — A thin rule along the device axes is fitted to one whole pixel under `/SA true`, and the fit is measured

Status: accepted, the construction and its figures; drawn by default in every backend since ADR 2553 (A714), off by default before it (ADR 2531). Session 1848, batch one hundred and twenty-five, the "measure" slot the owner's "10.7.5
let's measure it" and "Having to implement something is not a blocker, even if expensive" (2026-10-27)
asked for. I worked in `scratchpad/r1848/`. Context: ADR 0848 (the coordinate fit declined on the
coverage-weighted reading of thickness), ADRs 0028, 1189 and 1622 (the departure and its grounding), ADR
0085 (the window's lanes resolve a stroke's width per placement at encode), ADR 2531 (the decision these
figures give).

## The clause

ISO 32000-2 §10.7.5:

> When stroke adjustment is enabled, the line width and the coordinates of a stroke shall
> automatically be adjusted as necessary to produce lines of uniform thickness. The thickness shall be
> as near as possible to the requested line width -no more than half a pixel different.

Figure 70's caption is "Rasterization without stroke adjustment", and the paragraph that introduces it
names the defect as a count of pixels: "[a] line of a given width can intersect with different numbers
of device pixels, depending on where it is positioned". On an anti-aliasing device that defect shows as
a rule drawn across two rows at partial intensity in one place and on one row in another — the blur a
reader sees on a table's hairlines. *Uniform thickness* is read here as the clause's first paragraph
states it, a whole number of pixel rows the same at every placement; the second sentence bounds what
that number may cost against the requested width.

## The construction (`pdf_render::grid_fit`)

One function in `pdf-render`, beside `Stroke::device_width`, where ADR 0848 section 4 said it would go,
called at the entry of `render-cpu`'s stroker so everything after it strokes the fitted path as though
the document had stated it. It answers `None`, and the command's own path is stroked, unless all of:

- the stroke's `/SA` is `true` (Table 57's initial value is `false`);
- its device width is under `FIT_BELOW`, 1.5 device pixels, so one whole pixel is within the clause's
  half a pixel of it — the promoted widths under half a pixel included, for their position;
- the transform stretches both axes alike, so one stroke width is one device width (§8.4.3.2);
- every segment, closing ones included, lies along a device axis within a sixty-fourth of a pixel —
  a diagonal or a curve keeps its coordinates and its coverage-weighted thickness, ADR 0848's answer;
- the fit moves something.

Then the width becomes one device pixel and each vertex moves, *across* the segments it ends, to the
nearest pixel centre: a vertex of a horizontal segment has its device `y` moved to `⌊y⌋ + 0.5`, of a
vertical one its `x`, a corner both, and a free end keeps where the document ended it. No coordinate
moves more than half a pixel, so no rule leaves the pixel its centre was in. `CpuRasterizer::
with_stroke_adjustment(true)` turns the fit on and its default is off (ADR 2531); off is HEAD's drawing
and the measurement's other arm.
Six unit tests in `grid_fit.rs` and the two arms of `stroke_width.rs`'s placement ladder hold it; a
fit forced off fails both of the ladder's `/SA` tests.

## The instrument (`pdf-model/examples/stroke_grid_fit_measure`)

Page one of each pdf.js document, interpreted; where it paints a stroke under `/SA true`, drawn at 1×
by `render-cpu` with the fit off and on, five interleaved draws each. It counts, over the fitted
segments three or more pixels long: **crisp**, a pixel of the stroke's solid colour (within three
levels) across the midpoint, where the colour is not already what lies three pixels beside it;
**shared**, a stretch where another command's axis edge runs within 1.5 pixels across and three along;
**seams**, along shared stretches, pixels in the rows beside the rule's that are a valley (64 levels
lighter than both neighbours across) in one arm and not the other — opened or closed. Calibrated by a
plant: two 0.6-pixel rules 1.2 pixels apart, whose fitted rows leave one white row between them, read
**80 opened** seam pixels, its whole 80-pixel length. Crops of the worst and best pages are kept in the
scratch directory, never committed.

## The figures

One `--clock --nice 0` hold, 69 s, the load 3.82 at the arms' start after the 60 s cap (habit 186),
no process holding the render node:

| | |
|---|---|
| first pages that stroke under `/SA true` | 16 (of the pdf.js corpus's first pages) |
| drawn differently with the fit | 7 |
| strokes under `/SA`, fitted | 778, 450 |
| fitted, by requested device width | 1 under 0.5 px, 441 from 0.5 to 1 px, 8 from 1 to 1.5 px |
| segments sampled; crisp unfitted → fitted | 347; **4 → 316** |
| stretches shared with another mark | 123 |
| seam pixels opened, closed | **2, 0** |
| pixels changed | 23 693, 20 996 of them on `issue14297.pdf` |
| draw time, sum of the 16 pages' least of five | 333.5 ms unfitted, 330.9 ms fitted |
| the corpus gate on the 7 pages, fit on, against HEAD's six arms | raster's frame digests unchanged; at 1× the mean error rises on 6 or 7 pages per lane and `issue14297.pdf` goes from agreeing to differing (CPU lane 0.1233 → 1.6956 levels of 255, worst tile 11.97, SSIM 0.956); at 4× nothing moves |

The pages and what the eye sees in their crops: `issue14297.pdf`'s financial tables (438 fitted strokes)
go from grey two-row rules to one black row, and its two opened seam pixels sit where a rule meets a
header fill's top edge, a row of the fill now under the black rule rather than a grey half-row above
it; `issue2761.pdf`'s box outline and `issue15629.pdf`'s orange rule become one solid row; `stamps.pdf`'s
double border is drawn as two crisp rows with the white rule between them where it was a blur;
`annotation-underline.pdf`'s underline is one solid row under the text. The nine pages that paint a
`/SA` stroke the fit declines — `bug1743245.pdf`'s 268 among them, curves and glyph outlines — draw
byte for byte as before.

**What the fit costs is the clause's second sentence, as ADR 0848 said it would**: 441 of the 450
fitted strokes were requested between half a pixel and one, and each is drawn one whole pixel, up to
half a pixel heavier, within the bound and no longer 0.0039 from the requested width. The rules read
darker: that is the trade the clause's first sentence makes, and the second sentence's bound is where
it stops.

## Not measured, and why

The window's lanes — `raster`'s CPU coverage, compute and device rasterisers — resolve a stroke's width
per placement at encode (`raster::stroke::resolve_width`, ADR 0085) and apply the half-pixel rule only;
fitting them is a build in the `raster` workspace, encoding the fitted path per placement, and is owed
to a round of its own (ADR 2531). Above 1× the fitted population shrinks with the scale, because a rule
is wider in device pixels; 1× is where the clause's "low-resolution display applications" are.
