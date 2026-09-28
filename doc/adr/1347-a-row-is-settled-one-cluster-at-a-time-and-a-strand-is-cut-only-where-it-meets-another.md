# 1347 — A row is settled one cluster at a time, and a strand is cut only where it meets another

Status: accepted. Session 1255. Amends ADR 1341 section 2 (which rows are walked, and how).
Context: ISO 32000-2 §10.7.4, §8.5.3.3.2, §8.5.3.3.3, §11.6.2; CLAUDE.md principle 2 and "on the
tension between 2 and 4"; ADRs 1082, 1341.
Code: `crates/render-cpu/src/area.rs` (`Accumulator::measure_row`, `Accumulator::rewrite`,
`RowScratch::cluster`, `RowScratch::walk`, `RowScratch::walk_a_pair`, `Strand`, `Edge::by_row`).
Tests: `area.rs`'s `a_row_the_filter_leaves_unwalked_reads_what_the_walk_reads` and the closed
forms beside it; `crates/render-cpu/tests/overlapping_portions.rs`, unchanged.

## 1. What ADR 1341 cost, and where

`callgrind_rasterise`, `RAYON_NUM_THREADS=1`, five draws, per function (habit 45): on
`issue14415.pdf` and `issue19802.pdf` the walk was ~460M and ~320M instructions of the rise, and
it was spent **per row**: every row of a suspect mark with more than two pieces was cut at every
height a piece began or ended, and a stroke's outline is flattened at a 256th of a pixel, so a
row of a curved stroke held ~16 pieces and ~14 sub-strips. The rewrite then deposited every
piece once per sub-strip (818 740 `crossing` calls against 169 255 for the accumulation).

## 2. What changed, and why each keeps the construction exact

1. **Clusters.** A row's pieces, and the horizontal edges strictly inside it, are grouped by the
   pixel columns they reach. The winding left of a cluster is the same at every height of the row
   — a piece that ends inside the row meets the next edge of its path at that point, which lies in
   a column both reach, so the two are one cluster and what one adds to the columns right of them
   the other takes away; a horizontal edge joins what it reaches. So a cluster is settled alone,
   from that winding, and its pixels are rewritten keeping the running sum right of it where it
   was. A cluster of one piece, or of two that are one polyline through a vertex or the two sides
   of a thin shape over the same heights without crossing, takes two adjacent winding values in
   each pixel, which the accumulator reads exactly; it is not walked.
2. **Strands.** Edges that continue one another through a vertex in one direction are one chain
   (`Edge::sort_into`), and a chain's part of a row is one strand. Two strands keep their order
   between the heights where one begins, ends, crosses **or meets** the other, so only those are
   cuts; a strand crossing another exactly at its own vertex is cut there by `meeting_height`.
3. **Runs.** A piece bounding the set over consecutive sub-strips on one side is deposited once:
   the closed form over the union of two stretches of one straight line is the sum of the two.
4. **Order carried** between sub-strips (an insertion sort on a nearly sorted order), a counting
   sort of the edges by row in place of a comparison sort, a closed form for the most common
   cluster (two pieces over one height range, `walk_a_pair`), and the walk out of line so that
   the rare suspect mark's code is not every mark's.

The reference the whole is held to is the row walked whole (a test-only switch):
`a_row_the_filter_leaves_unwalked_reads_what_the_walk_reads` draws 3000 marks — random polygons,
some snapped to quarter pixels so that vertices, horizontal edges and coincident edges land on
boundaries, and many-sided loops standing for flattened curves — under both rules, and every
pixel agrees to one level; 60 000 marks were run once while building. Planted away: walking no
cluster fails it and four of the closed forms beside it; starting every cluster from a winding of
zero fails two closed forms; a strand not cut where it meets another at its own vertex failed it
at mark 1236 (84 against 86) before `meeting_height` existed.

## 3. What it moved

`raster_golden`, isolated from the batch's other changes: every page this construction moved
moved **one level** in a handful of pixels (at most 304, `issue17056.pdf`) — pixels both
constructions measure exactly, which the accumulator and the boundary sum round to adjacent
levels. None moved further. (ADR 1348's stroke construction moved the rest of the 35 pages.)

## 4. What it costs now

| page | before ADR 1341 | ADR 1341 | now | against 1341 | against before |
|---|---|---|---|---|---|
| `issue14415.pdf` | 1 470 834 624 | 2 162 981 897 | 1 926 623 979 | −10.9% | +31.0% |
| `issue19802.pdf` | 1 100 108 801 | 1 661 572 112 | 1 307 007 001 | −21.3% | +18.8% |
| `issue20232.pdf` | 363 423 489 | 400 004 759 | 423 155 667 | +5.8% | +16.4% |
| ISO 32000-2 p. 101 | 1 417 974 931 | 1 382 308 143 | 1 383 967 984 | +0.12% | −2.4% |
| `issue12295.pdf` | 8 341 335 329 | 8 383 397 663 | 8 433 577 656 | +0.60% | +1.1% |

`issue14415.pdf` includes ADR 1348's pieces (+55M; the walk alone reads 1 870 910 411).
`issue12295.pdf`'s +0.60% is +0.22% of work (the walk out of line) and +0.38% in `libc`'s
allocator at unchanged call counts, which is layout (habit 45). **The contract's target — within
a few per cent of the pre-1341 cost — is not met**, and the reason is measured: 72% of the
clusters walked on `issue14415.pdf` (3311 of 4590) do leave the range and are rewritten, so what
is left is the set's own boundary being computed where it differs from the integral.
`issue20232.pdf` is dearer than under ADR 1341 because nearly every walked row there is one small
cluster, and the cluster's own bookkeeping is what that page pays.

## 5. What it does not do: every mark

Settling every row of every mark by clusters would close ADR 1341's residue — an overlap confined
to pixels a path that is not an outline only partly covers, with no whole winding of two and no
sign against another — and it was measured: page 101 +114%, `issue14415.pdf` 17× for one
page-wide fill. The residue stays, bounded by the overlap's own area inside the pixel, and
`area.rs` names it where the suspect mark is decided.
