# 1479 — A residue link is flattened once a frame, and a small tile reads the edges in its rows

Status: accepted. Session 1322. Answers ADR 1467 section 3 (`bug1721218_reduced.pdf`'s 3.7 s a frame)
and `doc/QUORRA_FEEDBACK.md` sections 60 and 61's measurement. Amends ADR 0049's finding that
memoising a chain's flattening was not worth taking: on this page it is most of the answer.
Supersedes nothing.
Context: ISO 32000-2 §8.5.4, §10.7.4; ADRs 0049, 1389, 1444, 1445, 1456, 1467, 1471; traps 62, 71,
73; habits 52–60; `doc/questions/A76`.
Code: `raster/crates/raster-gpu/src/encode/residue/flats.rs` (new: `LinkFlats`, `LinkKey`),
`raster/fill/rows.rs` (new: `RowIndex`), `raster/fill.rs` (`fill_mask_indexed`, `fill_over`,
`deposit_at_border`), `raster/fill/topology.rs` (`plainly_two_values_among`, `Topology::of`'s
candidates), `raster/meet.rs` (`partial_windings`), `encode/clips.rs` (`flatten_chain`,
`intersect_links`).
Tests: `raster/fill/rows.rs` (a tile filled from its rows is the tile filled from every edge, both
rules, 2 000+ tiles of six sizes; watched failing with one row dropped from the listing),
`encode/residue/flats.rs`.

**`crates/render-cpu/` was not opened.**

## 1. Measured before anything was built

A probe in `Encoder::residue_intersection` on one `zoom_frame` of the page (removed). The page is drawn
as ADR 1471's two own-space frames; **each is one `Encoder`** — groups are planned inside it, not by
encoders of their own, so ADR 1467's guess that each group's shadings were planned by an encoder of their own does not
hold. Per frame there are 6 770 residue asks over **3 515 chains, by id and by content alike** (each chain one link, every
outline distinct): 3 513 asked once or twice, each a small region admitted at once; one chain of
**111 677 points** (a 207 × 120 region) asked **3 025 times**; one of 9 512 points asked 233 times.
Every one of those asks is a tile of 2 × 1 to 3 × 2 pixels — one per shading, each under a
rectangular clip of its own whose parent is the big path, so the deepest residue link, the region
cache's key, is already one id. **That is why keying the cache by content did not pay**: there was no
repeat for content to find. The region was declined by ADR 0049's own rule (24 840 bytes against
3 025 × 4), so each ask flattened 111 677 points again and filled every one of their edges over a
two-pixel tile.

## 2. The levers, in the order built, each measured (callgrind, one frame pair; `zoom_frame` minima)

| step | instructions | wall |
|---|---:|---:|
| HEAD (ADR 1467's profile) | 488 G | 7 248 ms |
| (a) a link's flattening kept per frame by `(outline, transform bits)` | 61.2 G | 2 382 ms |
| (d) the kept flattening's edges listed by row; a tile reads its rows' edges, sorted | 19.2 G | 975 ms |
| the rows' lists merged in order instead of sorted | 16.0 G | 795 ms |
| the topology question asked of the listed subpaths only | 15.0 G | — |
| an edge listed under the rows it crosses, not widened by one each way | 12.9 G | — |
| the exact meet's left partial edges added to bands by two searches, not tested per band | 12.5 G | 587 ms |
| a slab wholly left or right of a tile deposited at the border directly | 11.0 G | 556 ms |

The index is built on a flattening's second use, not its first (trap 73): a link asked for once —
the common clip — pays nothing for listing its edges.

(b) a region cache across encoders and (c) a stack of repeated links collapsing to one **do not
apply**: one encoder per frame, and every chain on the page is one link.

**Byte-identical, and why.** The memo hands back the polylines a miss makes (`flatten` is
deterministic, ADR 0008). The index changes which edges are visited, never what one deposits or the
order deposits arrive in: `Edge::cut` keeps an edge for rows `top .. top + h` only where its ends
satisfy `hi > top` and `lo < top + h` (rounding is monotone and `0`, `h` are exact), which is exactly
the rows it is listed under, and the listed edges are merged in the plain walk's order, so every `f32`
sum is the same sum. The topology candidates are the subpaths the box test would not skip. The border
deposit is `deposit_inside`'s own two products for a piece both of whose ends it clamps onto one
column. The meet's windings are integers. Corpus digests below.

## 3. The floor, argued

556 ms a page against the CPU backend's 84 ms. What is left of the residue path is the per-tile fill
(about 170 ms over both frames) and the exact meet (about 120 ms); the rest is interpretation, the
four-component ink table and the frames' other work. Admitting the big chain's region (forced, for
measurement only) draws the page in 387 ms — but a crop of a region and a tile filled alone differ by
ADR 0049's accumulator order, one level on rare pixels, so it is not byte-identical and is not taken
here. **The next levers**: ADR 0049's admission rule pricing a tile's edges as well as its bytes (a
byte decision, to be made with the corpus beside it), and the exact meet counting a pixel's left
winding from whichever side holds fewer partial edges.

## 4. Corpus

`render-raster --test corpus`, HEAD and this change built from exported trees with their own target
directory, per-page frame digests (`PDFVIEWER_RASTER_TIMES`): **0 pages differ** at 1× and at 4× on the
CPU, compute and GPU coverage lanes; one-versus-many 0 in all twelve runs; verdicts unchanged (1×:
966 agree / 0 differ / 1 refused on CPU and compute, 965 / 1 / 1 on GPU as before; 4×: 962 / 0 / 4,
GPU 961 / 0 / 5 as before). Raster's corpus total at 1×: CPU lane 15.6 s → 11.6 s, compute 16.8 → 8.8,
GPU 14.4 → 7.5; at 4×: 26.6 → 22.0, 26.6 → 20.0, 26.2 → 19.4 — `bug1721218_reduced.pdf` alone
7.0–8.9 s → 0.50–0.77 s, every other page within the walk's noise (the one 40× outlier, the
`tracemonkey` family on one run, drew 12–14 ms re-run alone).
