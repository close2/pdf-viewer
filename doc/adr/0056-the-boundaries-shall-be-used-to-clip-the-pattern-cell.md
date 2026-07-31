# ADR 0056 — "These boundaries shall be used to clip the pattern cell"

Status: accepted, 2026-07-31.

## Context

`CONTRADICTED_UNEXPLAINED` had 36 pages. The handover's instrument for choosing among them is a
ratio rather than a distance, and this session ran it over every entry: **our worst measurement
divided by the bound that measurement is held to**, taking the largest of mean, worst tile and
SSIM.

One page came out at **25.7×**, and the next at 3.2×:

| | mean / bound | worst tile / bound | differing |
|---|---|---|---|
| `tiling-pattern-large-steps.pdf` page 1 | 1.60 / 1.00 | **128.49 / 5.00** | 0.79% |

A page whose *mean* is 1.6 levels and whose worst tile is 128 of 255 is not anti-aliasing and is
not a colour conversion. It is a region drawn by one implementation and not by another, and the
mean is small because the region is small next to a 4000-point page.

## The page, and what it says

983 bytes, and every one of them legible. A 4000 × 400 page fills `50 50 3950 350 re` with a
tiling pattern whose cell is

    /DeviceRGB cs 1 0 0 sc 50 50 3950 300 re B

inside a dictionary stating `/BBox [0 0 3950 350]` and `/XStep /YStep 90000` — steps so large
that exactly one cell is painted. The cell's rectangle runs from x = 50 to x = 4000; the box
ends at x = 3950.

Sampling the five renders along one row settles it without any tolerance being involved:

| at x = | 3949 | 3950 | 3951 | 3999 |
|---|---|---|---|---|
| poppler | red | nearly white | white | white |
| ghostscript | red | white | white | white |
| `hayro` | nearly red | nearly white | white | white |
| **ours** | red | red | red | half red |
| `mupdf` | red | red | red | half red |

Three implementations stop at 3950. We and `mupdf` run to the end of the page.

Table 74 says which is right, in one sentence:

> These boundaries shall be used to clip the pattern cell.

**This tree carried no `/BBox` on a tiling pattern at all.** `Tiling` held the content, the
resources, the step, the matrix and the uncoloured tint, and nothing else. Nothing was clipped.

## Decision

`Tiling` carries the box, and every cell is clipped to it — `rect_clip` under the path's own
clip, so a cell is bounded by both, translated by that cell's own offset.

**Per cell, not per fill**, and the reason is what the clause is for: a cell whose content runs
past its own box would otherwise spill into its neighbour's box, and where `/XStep` exceeds the
box — which is precisely how a pattern tiles with gaps around each figure — it would spill into
the gap between them. A single clip over the union of the boxes would let both through.

Two edges are decided where they are made. A box the file writes with its corners in the other
order is normalised, exactly as a page box is. **A box with no extent is left unclipped rather
than clipping everything away**, because Table 74's NOTE 1 says "[a] BBox of zero height or
width will still paint one pixel" — emptying the cell would be the one reading the clause
forbids outright.

## The finding behind the finding

**The conformance ledger's row for §8.7.3.1 has said "`/BBox` clips the cell" since the twentieth
session.** It was written from the clause, it was never true of the code, and no test asked. The
row is `partial` for a different reason — `/TilingType` — so the status never looked wrong either.

That is a failure mode this project has not recorded before, and it is the mirror image of ADR
0054's: a row can be wrong by *claiming* as well as by *disclaiming*. The corrected row now names
the test, and the general rule is worth keeping in the ledger's own terms: **a note is a claim,
and only a test makes it a fact.**

## Consequences

- 817 pages agree with the reference consensus and 81 are contradicted, from 816 and 82.
- `CONTRADICTED_UNEXPLAINED` is 35.
- The corpus gate's wall time is unchanged within its run-to-run spread (2.5–2.9 s across runs
  before and after), and no corpus document reports a new limit: the extra clip is one rectangle
  per cell, under the same `MAX_TILES` bound of 4096 that already governs how many cells a fill
  may draw.
- `mupdf` agreeing with us is not evidence either way, and it is worth saying plainly: it is one
  implementation reading the clause as we did, and the clause is what changed our mind.
