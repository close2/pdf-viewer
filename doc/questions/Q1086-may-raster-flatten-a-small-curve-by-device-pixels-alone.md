# Q1086 — May raster flatten a small curve by device pixels alone? Option 3 of Q1073, priced

Source: round 2265 (ADR 3761), answering A1073's "price 3 next".

## What the tree does now

Raster flattens a cubic to the tighter of a quarter pixel and 1/32 of its own control polygon's
diagonal (`RELATIVE_FLATTEN_TOLERANCE`, raster's ADR 0044). The relative term binds only on curves
under 8 device pixels across, and there it keeps sixteen pieces a turn however small the curve. It is
raster's alone: the CPU oracle flattens to an absolute 1/256 px (1/20 px on a folding stroke) and
`render-gpu` (vello) to an absolute 0.25 px. ISO 32000-2 §10.7.2 lets a processor choose its flatness
("PDF processors may choose to ignore any flatness tolerance specified within a PDF file") and asks only
that the result not be "inscribed polygons"; since ADR 1443 each flat piece encloses its own area, so
dropping the relative term on a small curve costs shape (about 0.05 px at most on a one-pixel dot) and
about 1 % of a sub-pixel dot's ink, not the 36 % that ADR 0044 was written against.

## The price (ADR 3761 has the method)

| switch: device pixels alone below | off | 1 px | 2 px | 8 px |
|---|---|---|---|---|
| Type 3 page turn, ms | 2386–2628 | as at 2 px | 896–1119 | 941–1524 |
| seventh / step, ms | 2370–2684 / 855–933 | — | 937–1265 / 473–578 | 938–1288 / 465–533 |
| instructions, one draw (G) | 62.3 | — | 35.0 | 35.0 |
| corpus first pages moved (of 957) | 0 | 522 | 593 | 616 |
| of which by 8 levels or more | 0 | 5 | 11 | 147 |
| moved pages farther from / nearer the CPU oracle | — | — | 379 / 21 | 494 / 14 |
| raster unit tests that fail | 0 | — | 2 (the dot's ink, the piece counts) | 2 |

`raster_golden`, the strips test and `render-gpu` cannot move: none of them depends on `raster-gpu`.
The pages that move are text pages, most by 1 to 6 levels on a glyph's rim; the worst are the Type 3
page itself (42 levels) and `issue14415` (41). Pictures: `~/scratch/agent/r2265/worst/`.

Even with the switch the page turns in about a second, thirty times the band it had before ADR 3653:
the exact meet of its overlapping pieces is still two thirds of the work.

## The ask

1. **Keep the relative bound** (the round's recommendation). The Type 3 page keeps the band option 1
   gave it; nothing moves.
2. **Adopt option 3 at 1 px**: the gentlest — 522 pages move, 437 of them by 1 or 2 levels — for the
   same factor of about 2.5 on that page: its curves are all under a pixel, and at 1 px it draws the
   very frame it draws at 2 px (the same 1 283 pixels moved, the same summed difference).
3. **Adopt option 3 at 2 px**, as priced in full above.

With the switch at 2 px the turn walk kept every other page in its band; the launch walk had two
page turns past theirs at a load of 31, which a control without the switch at lower load did not —
not separable from these two runs, so adopting owes that comparison at equal load (ADR 3761).

Whichever is chosen, the two raster unit tests and the turn band of the Type 3 page move with it, in
the round that lands it; the prototype is `~/scratch/agent/r2265/option3-switch.patch`.
