# Q714 — Shall a thin rule under `/SA true` be drawn fitted to the pixel grid?

Source: round 1848, the "measure" slot of your "10.7.5 let's measure it" (2026-10-27), which you
asked after asking whether `/SA true` is the document's explicit request to be displayed with the fit
even where it could open seams (ADRs 2530 and 2531).

## Where it stands

§10.7.5's first requirement — "the line width and the coordinates of a stroke shall automatically be
adjusted as necessary to produce lines of uniform thickness" — is now built in `render-cpu`
(`pdf_render::grid_fit`): a stroke under `/SA true`, narrower than 1.5 device pixels and lying along
the device axes, is drawn one whole pixel wide across pixel centres. It is **off by default**, the
row stays `departed`, and nothing a reader sees has changed.

## The figures (page one of the pdf.js documents, 1×, fit off against fit on)

| | |
|---|---|
| pages that stroke under `/SA true`, drawn differently by the fit | 16, 7 |
| strokes under `/SA`, fitted | 778, 450 |
| sampled segments drawn as one solid row (hairlines made crisp) | 4 unfitted, **316 fitted**, of 347 |
| seam pixels opened, closed, over 123 stretches shared with another mark | **2, 0** (one page, `issue14297.pdf`) |
| what it costs in weight | 441 of 450 were requested 0.5–1 px and are drawn 1 px: up to 0.5 px heavier, the clause's bound exactly; unfitted they are 0.0039 px from the request |
| draw time, the 16 pages' least of five, summed | 333.5 ms unfitted, 330.9 ms fitted |
| pixels changed | 23 693, of them 20 996 on `issue14297.pdf` |

The crops (financial tables going from grey two-row rules to one black row; a double border drawn
as two crisp rows with white between) are kept in the round's scratch directory, uncommitted.

## What the window's lanes owe before the fit is whole

`raster`'s three lanes resolve a stroke's width per placement at encode and apply only the
half-pixel promotion (ADR 0085). Turned on in `render-cpu` alone, the oracle disagrees with every
lane on those pixels: measured on the corpus gate, `issue14297.pdf` goes from agreeing to differing on
all three lanes at 1× (mean error 0.1233 → 1.6956 levels of 255 on the CPU lane), the other six pages
stay inside the band, and at 4× nothing moves. With the fit off, as the tree has it, the six arms
match HEAD's on every page. Turning it on is therefore one round in the `raster`
workspace: the same fit at the four `resolve_width` call sites of `raster-gpu`'s encoder, per
placement, and a test that the lanes and the oracle agree on a fitted rule at eight placements.

## Options

1. **Draw it under `/SA true`, in every backend at once** — a `raster` round builds the lanes' half,
   then the default turns on everywhere and the row moves to `implemented`.
2. **Draw it in `render-cpu` now** and in the lanes later; the oracle and the window disagree on the
   seven pages meanwhile, and the corpus gate counts `issue14297.pdf` as differing.
3. **A reader setting** (off by default, or on by default), in the place a host supplies.
4. **Take it back**: the module and its tests are removed and the row stays `departed` as before.

**Recommendation: option 1** — `/SA true` is a value the producer had to write against Table 57's
`false`, the figures show the fit doing what the clause names (316 of 347 crisp) at a cost of 2 seam
pixels and no time, and drawing it in one backend only would break the oracle's agreement with the
window.
