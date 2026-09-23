# 1237 — A page is separated into a plane per spot ink, in the model

Date: 2026-09-23. Branch: `batch-1233-1238`, worktree shared with five sibling rounds.
ADR: [1311](../adr/1311-a-page-is-separated-into-a-plane-per-spot-ink-in-the-model.md).

## The census first

`pdf-model --example spot_depth` (new) over the crawl, first ten pages of each document: 88 890
documents, 374 238 pages, 8 517 pages naming a spot colourant in 2 931 documents. The count falls
away geometrically from one; the largest ordinary page names 49, and past it only six files of one
Ghostscript bug report at 1090. `colourants::MAX_SPOT_PLANES` is sixteen, forty-eight colourants.

## §10.8.3 stage two

Under the reader's simulation a page naming a spot colourant is interpreted once per plane of the
simulated device — `Interpretation::separation`, `colourants::Separation` — beside the page a
backend draws, which does not move. `ColourSpace` carries its colourant names; `Plane::Spot(n)`;
`DeviceSpots` rides in `Compositing::Subtractive` and its `paint` is §11.7.3's "additive value of
1.0" paragraph, with `All`, process-named separations, `NChannel` per-component evaluation, groups
passing spot planes through, soft masks carrying none, and §11.7.4.3's overprint one plane wider.
A page with no `/CS` is separated on the press §10.8.3 step a) consults. A mark in a colourant past
the bound reverts and is named per operator. Eleven fixtures in `tests/spot_planes.rs`, five of
them calibrated by planting `available` false.

## Measurement

callgrind, `HEAD` exported against `HEAD` plus this round's hunks, one sitting. Page 101 ×50:
1 250 231 227 → 1 249 666 364 (−0.045%). A 3000-mark `/DeviceCMYK` page: 46 607 061 → 46 532 370
(−0.16%). The first build paid +0.10%, all in `show_text`: `overprint_blend`'s inline test had
become two; the diff per function found it, and the fast test is now the overprint parameter.
`raster_golden` held 974, moved 0; `pdf-model` and `render-raster` corpus gates passed.

## Rows

§10.8.3 stays `partial`, its note narrowed to stages three and four; §8.6.6.4 and §8.6.6.5 name
the carried colourant names. Table 275's answer names the render side.
