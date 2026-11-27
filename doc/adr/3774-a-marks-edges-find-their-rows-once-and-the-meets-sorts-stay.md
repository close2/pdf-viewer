# 3774 — A mark's edges find their rows once, and the meet's sorts stay stable

Session 2268. Status: accepted. Context: the turn gate's `ContentStreamNoCycleType3insideType3.pdf`
p1 (ADR 3705, Q1073, A1073, Q1086); round 2251's list of exact savings left in the meet; the
principle that an optimisation is justified by its figure.

## The measurement

Callgrind, the page's first frame at zoom 1 through raster's shipped options and through the CPU
oracle, one rayon thread, the `gates` profile; every variant's frame digest is the baseline's
(`c3170ecb6e61c58d` raster, `eacb4672280f5136` oracle), so each figure is a cost and not a picture.

| variant | raster Ir | |
|---|---|---|
| main | 63.38 G | `area_in_pixel` 21.6 G, `Boundary` sorts 12.6 G, `RowEdges::build` 7.5 G + `floor`/`ceil` 1.6 G |
| every meet sort unstable on a unique key | 66.86 G | the sorts grew: the runs and the rows arrive nearly in order, which the stable merge sort finds in runs and quicksort does not |
| boundaries laid out in the band before's order, then the stable sort | 61.33 G with the change below | sorts fell 12.6 → 5.2 G and the layout added 4.9 G to `area_in_pixel`: no net saving |
| **each edge's rows found once in `RowEdges::build`** | 61.94 G (−2.3 %) | `floor` and `ceil` are calls into the C library at the target's baseline, and `span` asked them twice per edge |

## Decision

Only the last is kept: `RowEdges::build` keeps each edge's `(from, to)` rows from the bound's
pass and `runs_by_row` reads them, the same two integers from the same arithmetic. The meet's
sorts stay stable; round 2251's unstable `(xm, index)` sort measured worse on this page, and
carrying the order between bands measured even.

The CPU oracle's `area::union` is not changed. `Pairs::compare` is called 10.6 M times and
passes 42.5 M edge pairs whose boxes meet to the exact crossing test, which is 9.3 G of the
oracle's 31.0 G; the box test itself is about eight instructions a pair. A prefilter of `b`'s
edges against `a`'s box found the same crossings in the same order and saved nothing (31.03 G),
and an `i64` path for determinants of near points cost more instructions (31.84 G). The pieces
genuinely meet: a thousand of them overlap in a few pixels, so what costs is the number of pieces,
which is Q1086's question and not a search's.
