# 2333 — `bug1743245.pdf`'s encode is the tight bends' tiling, and the two levers under it stay dropped

Status: rejected — the two levers priced here are not built; the attribution is the measurement that
priced them. Session 1749, 2026-10-25, the measure slot of batch one hundred and fourteen. Context:
ADRs 1375, 1389, 1421, 1431, 1444, 1505 (the tiling, its costs and the levers tried on it), 2314
(the glyph atlas's two thirds of a toolkit notch), 1969 (when a band moves); habit 164;
`doc/checks/turn-path.toml`'s row. Moves no ledger row and no band.

## 1. The question, and the hypothesis that did not hold

`bug1743245.pdf`'s turn is 40 to 43 ms in the turn gate's band notes, about 32 ms of it raster's
encode, on 269 commands. The brief's hypothesis was that the encode is the glyph atlas, the term ADR
2314 found to be two thirds of a toolkit notch. **It is not**: the page has no font (`frame_budget`
prints "fonts empty"), and the atlas does not appear in the profile. The encode is §8.4.3.2's set of
a tightly bent stroke made a tiling (ADR 1375), and the fill after it.

## 2. Where the encode goes, counted

Callgrind over one round of `render-raster`'s `examples/frame_budget` on an export of HEAD (release,
the device present; the turn, warm, seventh and step frames, of which three encode):

| function | instructions, three frames | share of the encode |
|---|---|---|
| `encode::parallel::rasterise` (the encode's fan-out) | 8 506 M | 100 % |
| `raster::stroke::disjoint::tile` (ADR 1375's tiling) | 5 749 M | 68 % |
| `raster::fill::fill_mask_settled` (ADR 1389's fill) | 1 924 M | 23 % |

So 2.8 G instructions a frame, divided across the fan-out's threads (23 started a frame). A second
build with the tiling's four inner functions held out of line (a probe, not kept) divides the
tiling: the cuts themselves (`Convex::subtract_from`, 1.63 M calls) 47 % of it, of which
`Line::split` is 29 %; the separating-line questions that decide whether to cut (5.17 M) 20 %; and
the tiling's own scan over earlier pieces with its bookkeeping the rest. The probe's out-of-line
calls cost 8 % more in all, so these shares are of the probe and not of HEAD's figure.

## 3. The two levers, at their ceilings (habit 164)

Both levers the profile names were built once already, timed pinned, and dropped (ADR 1505 section
3). The bar for reopening one is a ceiling that has grown since it was dropped, and neither has: ADR
1505's probe put the scan at 939 M of 7 621 M, 12 %, where it is about 8 % of the encode now, and
the buffered split was measured dearer outright.

- **The scan made free.** In HEAD's inlined build the box test that passes over an earlier piece and
  the loop around it are about 680 M of the tiling's 5 749 M, 8.0 % of the encode. ADR 1505's scan
  that finds the next meeting box saved 370 M and read 0.6 to 1.6 ms slower pinned, and ADR 1431's
  sweep cost more in its sorting than it saved. Not built.
- **Each point's side computed once in `Line::split`.** A cut computes a point's side of the line
  once for the polygon's range and twice more in its edge loop. Every side the tiling computes, in
  the split and in the questions, is 413 M, so two thirds of them is at most 3.2 % of the encode,
  and ADR 1505 measured the buffered form at 510 M more, not less. Not built.

## 4. What would move the row

The cutting is the cost, and it is exact by construction: every fragment is a piece less the pieces
before it. Fewer cuts means a different tiling — fewer, larger fragments with other crossing points
— which moves the rim pixels' rounding and so is a change with pixels and an ADR of its own, priced
in none yet. Nothing in this round changes the tiling, and the row's band stands.
