# 2531 — The grid fit stays off by default, and §10.7.5 stays `departed`, until the owner chooses

Status: accepted. Session 1848, batch one hundred and twenty-five; the reading of ADR 2530's figures,
amending ADR 0848's measurement. I worked in `scratchpad/r1848/`. Question: `doc/questions/Q714`.

## What the owner asked

"10.7.5 let's measure it" (2026-10-27), after asking whether `/SA true` is the document's explicit
request to be displayed with the fit even where the fit could open seams. The measurement is the
deliverable; whether the fit is drawn is the owner's, and Q714 puts it to them with the figures.

## What the figures say (ADR 2530; page one of the pdf.js documents, 1×)

- **The fit does what the clause's first sentence names.** Of 347 sampled segments on the 7 of 16
  pages it changes, 316 are one solid row fitted against 4 unfitted; on the placement ladder the
  fitted rule is one whole pixel at all eight placements, spread at most 0.02 of a pixel.
- **Seams are rare where they were feared.** Of 123 stretches a fitted rule shares with another
  mark's edge, 2 pixels opened, on `issue14297.pdf`, and 0 closed; the instrument reads 80 of 80 on a
  plant made to open them, so the 2 are not a blind instrument.
- **It costs the second sentence's margin, not time.** 441 of the 450 fitted strokes were requested
  between half a pixel and one and are drawn one whole pixel, up to half a pixel heavier — inside the
  clause's bound, where unfitted they are 0.0039 from the requested width (ADR 0848). Draw time, the
  sum over the 16 pages of the least of five: 333.5 ms unfitted, 330.9 ms fitted.
- **The oracle would stand alone.** `raster`'s lanes resolve a stroke's width per placement at encode
  and apply the half-pixel rule only (ADR 0085), so with the fit on in `render-cpu` alone every lane
  disagrees with the oracle on exactly the pixels the fit moves — 23 693 at 1×, 20 996 of them on
  `issue14297.pdf`. Measured, on the corpus gate built with the fit on and run on the 7 pages: the
  lanes' digests do not move, the mean error rises on 6 or 7 pages per 1× lane, and `issue14297.pdf`
  goes from agreeing to differing on all three (0.1233 → 1.6956 levels of 255 on the CPU lane, 0.0201
  → 1.5950 on the compute lane); at 4× the fit moves nothing on these pages.

## Why the figures do not decide it alone

Both constructions meet the clause's bound, and the clause never defines *thickness* (§10.7.5's
ledger row): under the coverage-weighted reading the unfitted rule is nearer the requested width
(0.0039 against up to 0.5 of a pixel), under the touched-row reading only the fitted one is uniform.
Which reading is "as its producer specified" when a producer wrote `/SA true` is exactly what the
owner asked, and it is a reading of a document's intent rather than a figure. **ADR 0848 is amended**
on its measurement, not its conclusion: the fit it priced on `poppler`'s ladder (0.4000 from a requested
0.6) is now this tree's own, measured on the corpus, and its cost is seams on 1 page of 7 rather than
an argument.

## The decision

- **The code stays in the tree, off by default.** `pdf_render::grid_fit` and its six unit tests,
  `CpuRasterizer::with_stroke_adjustment` (default `false`) and the two arms of `stroke_width.rs`'s
  ladder stay, because they are the construction either answer to Q714 starts from: an answer of
  "draw it" turns the default on and builds raster's half, an answer of "do not" deletes the
  module's 316 lines and about 30 in `render-cpu` in one pass. Nothing outside the tests and the
  instrument turns the fit on. The cost of keeping it is one boolean asked at the stroker's entry.
- **The six arms match HEAD's**, which is the condition for keeping it: with the default off, the
  corpus gate's six arms (three lanes at 1× and 4×) on my export agree with HEAD's six arms on every
  page, 968 at 1× and 965 at 4×, frame digest and mean error both, every arm exit 0; and
  `raster_golden`'s walk holds 974 tracked first pages with 0 moved.
- **§10.7.5 stays `departed`.** Its note names the build, its default and Q714; the status moves to
  `partial` or `implemented` only with the owner's answer and raster's half.

## What turning it on owes

The same fit in `raster-gpu`'s encoder, at the four places that call `raster::resolve_width`
(`encode/expansion.rs`, `encode/stroke.rs`, `encode/line.rs` twice), per placement because one
scene is drawn at every magnification (ADR 0085): the vertices moved to pixel centres there, a test
that the three lanes and the oracle agree on a fitted rule at eight placements, and the corpus gate's
mean errors on the seven pages back to HEAD's. That is a round of its own in the `raster` workspace.
