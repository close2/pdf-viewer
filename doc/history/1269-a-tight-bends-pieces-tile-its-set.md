# 1269 — A tight bend's pieces tile its set

Batch forty-one, raster slot. No ledger row moved (§8.4.3.2–§8.4.3.4 were `implemented`; this made
raster's side meet them where a bend is tighter than the half-width). ADR 1375, feedback section 55.
`crates/render-cpu/` was not opened.

## Measured before building
The hook's shortfall (557.87 against 565.50 at 1×) is not a double count: its pieces' union is
557.83, the rest the flattened centre line. The defect is real on a short tight bend: a quarter
circle of radius 3 at `8 w` read 39.51 against its union's 38.27, one pixel 0.37 over.

## Built
`raster-gpu/src/raster/stroke/disjoint.rs`: the pieces that meet a vertex `inner_cut` declines (and
any piece whose box meets theirs) are re-cut into `P_k − (P_0 ∪ … ∪ P_{k−1})`, convex minus convex by
half-planes, nearest piece first; separated pairs are skipped by a separating edge. Bounded at 1 024
pieces and 65 536 fragments, past which the overlapping pieces stand. After: the arc 38.28, the
hook 557.86 → 564.30, worst pixel against the union 0.006 (sampling).

## Fixtures (`stroke_set.rs`)
A tight L (arms 2, `8 w`): every pixel within one step of inclusion–exclusion over its rectangles at
1×–8× both ways; pieces sum to 44 / 36 (planted away: 48, ink 44.47). A round-capped dot of `4 w` is
its disc within ADR 0044's 2.55% at 1× and 4× (the 0.35 step kept: a distance-only step is an
octagon, 11.38 against 12.63, planted).

## Corpus and cost
`render-raster --test corpus` 958 agree / 2 differ (with 1270's password pages), nothing moved.
`ContentStreamNoCycleType3insideType3.pdf` at 1× (every bend tight, one glyph subpath tiled per tile
placement): about 10 ms → 47–93 ms through raster, the CPU backend 58–93 ms; the corpus total within
the run-to-run noise of both arms. `issue10572.pdf` at 4×: the same on all three lanes; against rows
from the file raster errs in 2 rows (its 4 096-texel ramp, half a texel) and the oracle in 7 — ask
written, oracle's handed on. `raster_golden` untouched: the CPU oracle did not change.

## Left
Section 45's fill case and untight crossings; a butt cap square to the last chord, not the tangent.
