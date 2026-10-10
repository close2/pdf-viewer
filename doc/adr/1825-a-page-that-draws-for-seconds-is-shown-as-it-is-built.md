# 1825 — A page that draws for seconds is shown as it is built, in painting order

Asked by the project owner on 2026-10-10, of the first frame on his drawing
`/tmp/demo_heavy_engineering_A1.pdf`: okular shows a page "step by step … better than waiting; but
not a requirement". Status: **accepted** and **built** in the two toolkit windows. Context: ADRs 0139,
0219, 0650, 0657, 0668, 0714, 1811, 1813. Code: `crates/render-cpu/src/glimpse.rs`
(`Glance`, `Glimpse`, `CpuRasterizer::showing`), `crates/viewer-host/src/drawing.rs`
(`GLIMPSE_AFTER`, `GLIMPSE_EVERY`, `Partial`, `over_frames`), both hosts' `refresh`/frame path.
Tests: `render-cpu/tests/glimpsed_draw.rs`, `drawing.rs`'s three. Drive: step 81 `page_as_built`,
and `--step page_as_built_heavy`. Measurement: `viewer-host`'s `examples/page_as_built`.

## 1. The brief's unit was a tile, and this page grants none

The contract asked for the visible tiles first, each presented as it lands. A tile of a page here is
a strip, and ADR 0139 admits a strip boundary only at a row no curve and no stroke crosses
(`pdf_render::unsplittable_rows`); a tile cut anywhere else draws other bytes than the page, by up to
a quarter of a channel on a curve. The owner's page strokes a frame round the whole sheet among 4.4 M
strokes, so it grants no cut and draws as one strip (round 1488's record). Tiles would have shown it
at its end, as before, or shown pixels the page does not have and then replaced them.

## 2. Decision: the unit is painting order

A display list's commands are independent and drawn in order, so the page after its first *k*
commands is a state the draw passes through. `render_cpu::Glance` is a counter another thread bumps
to ask for that state; each strip checks it once per top-level command (one relaxed load, beside the
interrupt's), and answers with a copy of its rows finished as the page is finished: §11.4.7's medium
under the marks, the grey curve or colour cube where the list has one, §14.11.2.1's crop, straight
alpha by `tiny-skia`'s own conversion. The draw writes nothing back, so the final raster is the
unobserved one byte for byte. A strip that ends while somebody looks answers once more with its
final rows, so on a page that does grant cuts the strips land one by one as well; that is the tile
shape, wherever ADR 0139 allows it. Not glimpsed, because a first plane alone would show colours the
page will not have (trap 5): §11.4.7's four-component blending space, §10.8.3's simulation, §11.7.5.2's
transfer functions. Those pages are drawn and shown as before.

`viewer_host::Drawing` asks on the poll it already runs: first at `GLIMPSE_AFTER`, 500 ms after the
draw was handed over (half of `WARN`, above the slowest of `doc/pdf.js`'s 957 first pages at 315.9 ms,
so no page of that population is ever looked at), then every `GLIMPSE_EVERY`, 250 ms. `Partial` holds
the rows for the request in flight only; it goes when the draw lands, is abandoned or is replaced, and
the `RenderReady` is unchanged. `over_frames` shows it where the arrangement places that page at the
size it is being drawn at, for the document in front, in place of whatever the viewer holds for the
page. The bar of ADR 1811 carries `Partial::fraction` (commands drawn, weighted by the rows each strip
owns) and goes with the draw. `first frame on the screen` stays the finished page; a picture part-way
is `page N shown as it stands, P % drawn`.

## 3. How others do it, as evidence about the approach

`doc/pdf.js` (2ea8820d9) runs a page's operators in 15 ms slices (`EXECUTION_TIME`,
`src/display/canvas.js`) and its viewer shows "the canvas as it has been drawn" between them, first
after `minDurationToUpdateCanvas`, 500 ms by default (`web/base_pdf_page_view.js`); it does so only
where no earlier canvas of the page exists, and keeps the scaled old one on a zoom. pdfium pauses
and resumes one page's rendering (`fpdf_progressive.h`: `FPDF_RenderPageBitmap_Start`,
`FPDF_RenderPage_Continue`). okular and pdfcraft render independent tiles. Here the toolkit windows hold no reprojected picture of a zoomed page —
`stale::Plan` is `quorra`'s — so a page part-way at its new size is shown in place of the old size,
a choice and not a finding; a later round that gives these hosts a reprojection should revisit it.

## 4. Measured

The witness at 1002x708 (`examples/page_as_built`, `--clock`, 4 rayon threads): first picture
0.503 s and 0.502 s after the draw began, 28 pictures, whole page 7.136 and 7.245 s looked at against
7.197 and 7.118 s unobserved, **+0.46 %**, rasters byte-identical. Window path, two alternating rounds
of HEAD's binaries and HEAD with this change, seconds by the window's trace (first page content;
finished page). Xvfb: `quorra-gtk` 9.64, 9.10 → **3.58, 3.12**; 9.57, 8.35; `quorra-qt` 10.06, 8.20 →
**3.97, 2.82**; 9.69, 8.60. Owner's display (XWayland, 890M, one clock hold): `quorra-gtk` 8.58,
9.15 → **3.14, 3.01**; 9.52, 8.59; `quorra-qt` 10.34, 9.95 → **3.97, 2.98**; 11.05, 12.79 — taken at
load 6–13, and the finished figures wander both ways. Re-taken at load 2, `quorra-qt` alone, three
alternating rounds: 9.46, 9.50, 9.47 → **2.88, 2.88, 2.98**; finished 9.54, 9.56, 9.67, its draw
7.06–7.12 s → 7.14–7.16 s (+1 %); with its bar not fed during the draw, 7.23–8.17 s, so the bar costs
nothing. The first picture lands about half a second after page one is read, which is
`GLIMPSE_AFTER`.

## 5. What it does not do

It does not move the read (2.3–3.6 s here, the core's rule 4 and slot 1's question) or the draw;
it makes the draw visible. `quorra`'s processor fallback composes the whole arrangement in
`viewer_ui::software::compose_pages`, owned by another slot this batch; `CpuRasterizer::showing` is
the road it would take. `quorra-confined` draws in the worker or from crossed marks, and neither
path asks yet.
