# 1503 — A run of short edges crosses a row as one edge

Status: accepted. Session 1334. Answers ADR 1491 section 4, which named `area_in_pixel` as "the
next lever". Amends ADR 1479's row index: a row now lists runs of edges, not single edges.
Supersedes nothing.
Context: ISO 32000-2 §8.5.3.3.2, §10.7.4; ADRs 1444, 1467, 1479, 1480, 1491, 1492; habits 52, 53,
56, 59, 61; traps 92, 94; `doc/questions/A76`.
Code: `raster/crates/raster-gpu/src/raster/meet.rs` (`Entry`, `Entry::continued_by`,
`runs_by_row`, `row_starts`, `RUN_WIDTH`, `sort_row`).
Tests: `raster/meet.rs`'s `a_finely_flattened_curve_meets_as_its_pieces_do` (new; a 3000-gon met
with a half-plane, both windings, both rules, every pixel of 144 against the convex closed form to
10⁻⁹; watched failing at 1.0 with a run's span left short).

**`crates/render-cpu/` was not opened.**

## 1. Where the 3.26 G went (callgrind, one `zoom_frame` pair of `bug1721218_reduced.pdf`, one thread)

A probe tree marked the meet's steps `#[inline(never)]` and counted per pixel. 45 734 pixels
are asked in the pair; each has 24 bands, 20 edges through it and **226 partial edges beside
it**. A partial edge spans part of the row, so it adds to some bands and not others. Of
`area_in_pixel`'s 3.29 G, `partial_windings` (two searches per partial edge) took 1.41 G and
`sort_row` (pushing them) 0.70 G. Band areas took 0.50 G, edge crossings 0.28 G and sorting the
cuts 0.26 G. Allocation per meet is 26 M of `exact_areas`' 2.05 G.

Why so many: the page's clip is curves flattened into pieces shorter than a row. Each piece
spans part of the row, so each was a partial edge, though together they cross the row once.

## 2. The lever kept: runs

A row lists **runs**: consecutive edges of one direction whose parts in the row abut end to
start. Abutting half-open spans partition their union, so a ray at any height of the union meets
exactly one edge of the run. The run adds what one edge over its span would. A run spanning the
whole row joins the row's prefix sum; any other run is one partial edge. A pixel that a run
reaches reads the run edge by edge, with the edge-level test it had before. Windings are
integers, the cuts are the same set and the crossings do not depend on order, so **every byte is
the same by construction** (section 4 checks it). The bucket bound still counts edge-row pairs,
so what a set may cost and when a meet keeps `min` do not move.

A run is capped at `RUN_WIDTH = 1.0` across its row. Uncapped, a shallow curve made runs dozens
of pixels wide that each pixel had to expand: `area_in_pixel` went to 3.67 G, a loss.

| cap | `area_in_pixel` | pair total |
|---|---:|---:|
| HEAD (no runs) | 3.25 G | 9.48 G |
| none | 3.67 G | 9.78 G |
| 0.25 | 2.22 G | 8.44 G |
| 0.5 | 1.91 G | 8.05 G |
| **1.0** | **1.79 G** | **7.89 G** |
| 2.0 | 1.79 G | 7.90 G |
| 4.0 | 1.79 G | 7.90 G |

Partial edges per pixel fall from 226 to 41.

## 3. The brief's levers, each measured

- **(a) Bands shared along a row-run: not built.** In a meet, a row holds 1.94 asked pixels on
  average (21 758 pixels in 11 188 meet-rows per frame). A pixel's bands depend on where its
  edges cross its own sides. There is no run long enough to share across.
- **(b) A wholly covered pixel decided before the bands: already applied.** `both_cut` sends a
  pixel to the exact path only where both bytes are fractional (ADR 1467). On this page, 0 of
  45 734 asked pixels have a set with no edge through them.
- **(c) The winding as a prefix sum: built as section 2.** Whole-row edges were already a prefix
  sum (ADR 1479). The per-band count was over partial edges, and runs turn most of those into
  whole-row entries.
- **(d) Scratch reused per row: not built.** All allocation in the meet is 26 M (1.3%).
- **Also tried and not kept.** Bucketing the through edges by band cost more than it saved
  (1.79 → 1.91 G). Hoisting each through edge's slope saved 1% (1.79 → 1.77 G), and clarity
  wins at 1%.

**Repeated asks.** Across the corpus at 1×, one walk asks 30 624 pixels in 7 613 meets on 19
pages. ADR 1467's 15 222 counted both rasterisers. Only 68 asks repeat a pixel under the same
mark and chain within a frame. 14 719 asks repeat a pixel under another mark, 12 930 of them on
`bug1721218`. A walk draws each page once, and the two frames of a zoom step differ in scale, so
nothing repeats across frames. The ten pages with the most asked pixels: `bug1721218_reduced`
21 758, `issue6961` 2 128, `issue19360` 1 156, `issue14297` 1 132, `issue17147` 881, `issue2177`
668, `bug1868759` 664, `pattern_text_embedded_font` 532,
`ContentStreamNoCycleType3insideType3` 488, `bug1771477` 469.

## 4. Corpus

`render-raster --test corpus` ran on the CPU, GPU and compute lanes at 1× and 4×. Each run used
an exported HEAD tree and an exported HEAD-plus-this tree, each with its own target directory and
different binaries (checked by `md5sum`). Per-page digests (`PDFVIEWER_RASTER_TIMES`) matched:
**0 pages moved on all six runs**. Verdicts are the same as HEAD's. At 1×: CPU and compute
966 / 0 / 1, GPU 965 / 1 / 1. At 4×: CPU and compute 962 / 0 / 4, GPU 961 / 0 / 5.
One-versus-many is 0 on all twelve runs.

## 5. What is left between the step and the CPU backend's 84 ms

The pair runs `rasterize_frame` at 5.17 G, about 2.6 G a frame. In the same pair:

| part | pair |
|---|---:|
| exact meet (`exact_areas`) | 2.05 G, of which `area_in_pixel` 1.79 G and the mark's `RowEdges::of` 0.23 G |
| the chains' region fills (`residue_intersection`) | 1.13 G, of which `intersect_links` 0.82 G over 14 060 chains, `flatten_chain` 0.16 G and `row_pieces` 0.11 G |
| the clip's row index (`residue_edges`) | 0.4 G |
| everything else in the frame | 1.6 G |

Inside `area_in_pixel` the order is now band areas 0.50 G, partial windings 0.34 G, row sorting
0.29 G, crossings 0.28 G and sorting the cuts 0.26 G. No single step dominates any more.
**The next lever is the clip's side of a pixel asked by more than one mark.** That is 59% of
`bug1721218`'s asks. Its cuts, its edges through the pixel and its partial windings do not
depend on the mark. Kept for the frame, the mark would add only its own. The other is the 14 060
small region fills, about 58 k instructions each. Measured wall time, as minima of five rounds in interleaved arms at load 3.4–3.7: the 1.25×
zoom step falls from **250 ms to 200 ms**, against the CPU backend's 84 ms.
