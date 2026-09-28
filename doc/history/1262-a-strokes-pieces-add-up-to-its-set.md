# 1262 — A stroke's pieces add up to its set, whichever way the path runs

Batch forty, raster slot. No ledger row moved (§8.4.3.2–§8.4.3.6 were already `implemented`; this
made raster's side meet them). ADR 1361. `crates/render-cpu/` was not opened.

## The clause and the rule
§8.4.3.2's set, "all points whose perpendicular distance from the path in user space is less than
or equal to half the line width", filled by raster as convex pieces under an integrated, clamped
winding. Two rules make the pieces add up to it: every piece wound the rectangles' way on both
turns (`join_at` visits `p2` first when `cross > 0`), and pieces meet edge to edge wherever the
half-width allows (`inner_cut`; at an exact reversal the shorter rectangle is not emitted). A round
join at a reversal is a half disc; arcs are cut to `FLATTEN_TOLERANCE`, capped at 256 steps.

## Fixtures (`raster-gpu/src/raster/tests/stroke_set.rs`)
Each drawn both ways at 1×, 2×, 4×, 8×, the two directions within 16/255 at every pixel: the hook
(565.50 sampled; 557.87 → 564.30), the circle (188.52; 188.17 → 188.50), a 64-gon (188.42 closed
form; within 0.06 at every rung), Table 54's three joins on an L (640 / 636.57 / 632), §8.4.3.5's
1.414 example, a reversal (345.13 round, 320 miter), a dashed L's caps and corner (442.89). A
signed-area sum of the pieces, with no rasteriser, meets 640, 632, `P·w` and 120 exactly. Planted
away, each of the four changes fails at least one.

## Corpus
`render-raster --test corpus` 946 agree / 5 differ → 949 / 2. Left:
`ContentStreamNoCycleType3insideType3.pdf` (5117.46 v 5121.42 at 1×, 5094.44 v 5094.61 at 8×; nothing behind it at the gate's
tolerance), `issue20232.pdf` (+32.3% at 1× → 17 926.58 v 17 932.18) and `issue15150.pdf` (0.17 v
0.17): both out-and-back subpaths, section 45's stroker instance. Stay, numbers refreshed:
`issue19083.pdf` (396.58 v 448.33 at 1×, section 24c) and `issue2177.pdf` (0.74% → 0.17%).
GPU lane at 4× (ratchets off): 947 / 1, `issue10572.pdf`, identical with the old stroker.

## Left
Section 45's fill case (a path stated twice) is not the stroker's. The pieces still overlap where a
curve bends tighter than the half-width. `raster_golden` untouched: the CPU oracle did not change.
