# 1467 — A residue clip meets a mark by the area of their intersection

Status: accepted. Session 1316. Builds what ADR 1456 priced; amends ADR 1444 on the path lane (`min`
stays wherever one set holds the other in a pixel, and on the image lane). Supersedes nothing.
Context: ISO 32000-2 §8.5.3.3, §8.5.4, §10.7.4; ADRs 0049, 1395, 1419, 1444, 1445, 1456; traps 54, 71,
78, 81; habits 50, 52, 53; `doc/questions/A76`.
Code: `raster/crates/raster-gpu/src/raster/meet.rs` (new: `RowEdges`, `area_in_pixel`),
`encode/meet.rs` (new: `Encoder::meet_residue`, moved out of `encode/coverage.rs`, `Mark`),
`encode/clips.rs` (`Encoder::residue_edges`), `encode/residue.rs` (the edge cache and its budget),
`encode/parallel.rs` (`Made`: a residue job's polylines carried to its commit),
`encode/parallel/commit.rs`.
Tests: `raster-gpu/tests/residue_meets_a_mark_as_a_set.rs` (a fill and a stroke, three arms);
`raster/meet.rs`'s unit tests (two crossing stars under the two rules, against the predicate).

**`crates/render-cpu/` was not opened.** Every expected value is a closed-form area or the rule's own
predicate sampled.

## 1. The clause and the construction

§10.7.4: "Subsequent painting operations shall affect a region that is the intersection of the set of
pixels defined by the clipping region with the set of pixels for the region to be painted." Where the
mark's byte and the clip's byte are both fractional, `min` is an upper bound (ADR 1444); elsewhere it
is exact. So only those pixels are recomputed, from both sets' edges: the pixel is cut into bands at
every height where an edge through it starts, ends, crosses a side of the pixel or crosses another
edge; each set's winding entering from the left is §8.5.3.3.2's ray count over that row's edges; and
walking each band left to right, the strips every set's rule calls inside are trapezoids summed in
`f64`. The per-pixel kernel was chosen over per-pixel Sutherland–Hodgman because a mark or a clip is a
winding set, not a convex polygon: overlapping glyph contours and a stroke's pieces under the
non-zero rule, even-odd holes. A row's edges are kept sorted by where they start across it, with a
running reach and a prefix sum of whole-row windings, so a pixel finds the edges that can touch it by
two searches (the first version walked the whole row per pixel: 1.33 s over the corpus at 1×; this
one 0.66 s).

**Both paths, or neither.** The one site is `Encoder::meet_residue`, which the walk's tile and the
fan-out's commit both reach on the walk's thread in encounter order; the fan-out job now returns its
polylines beside its mask where its draw carries a residue, so the commit has the mark's edges
whatever the thread count. The chain's edges are built once per chain over its region and kept for the
frame under a budget equal to and separate from the regions' (so keeping edges never changes which
regions are admitted); a chain past that budget is flattened again per meet over the meet's rows, and
a row's bucket holds every edge reaching it either way, so a budget changes only cost, never bytes.

## 2. Measured

ADR 1444's fixture (64-gon over 99 rectangles) and a new one (five slanted butt-capped strokes under
the same 64-gon): every pixel within 0.5 of a level of the closed form — one rounding — on the
fan-out's commit at one thread and four and on the walk's own tile (`Coverage::Gpu` keeps a
residue-clipped mark there), the three arms byte-identical. Under `min` both failed (watched): pixel
(6, 2) drew 55 against 12.06; a stroke pixel 33 against 0.

Corpus at 1×, one binary with the exact meet switched off by environment (removed): 15 222 meets have
a doubly-fractional pixel; 56 858 such pixels, 43 066 lowered from `min`, by 990 360 levels in all
(23 a pixel). The gate: 965 agree / 0 differ in both arms, one-vs-many 0. Nine pages' frames moved;
against the oracle 8 further, 1 equal, none past the gate (`bug1721218_reduced.pdf` 0.0054 → 0.0147,
`issue2177.pdf` 0.493 → 0.501, `issue14297.pdf` 0.116 → 0.125, `pattern_text_embedded_font.pdf`
0.0063 → 0.0099, five by under 0.001). Further from the oracle is the expected direction (habit 50):
the oracle meets by a function of two bytes, and the fixtures hold the new value to the geometry.

**Cost.** The exact-meet step itself, timed per meet over the corpus at 1×: 0.66 s in all, of which
0.64 s is `bug1721218_reduced.pdf` (four frames of about 160 ms, 6 769 meets and 10 879 pixels each)
and no other page above 6.7 ms. Callgrind on that page's frame (488 G instructions): `meet_residue`'s
own work is 1.8%, `RowEdges::of` 0.24%. **Kept**: the cost is bounded by the rim pixels, and the page
that pays most pays for a separate defect (section 3).

## 3. `bug1721218_reduced.pdf`'s 3.7 s, priced and not this ADR's

The callgrind above: 96.6% of the frame is `residue_intersection` — `flatten_chain` 61.5%
(`flatten_cubic` 40.5%, the polylines' `Vec` growth 21%), `intersect_links`' fill 33.2%. The page sets a
clip before each of 3 518 shadings; the scene gives each `W` its own clip id, so 40 632 chain
flattenings of about 50 000 points each are made for two distinct outlines under one transform
(9 078 ids apiece), and each chain's region is declined as larger than its single use and rasterised
per tile. That is ADR 0049's cache keyed by id, before this ADR. Keying the cache by the chain's
content (outline, transform bits, rule, parent) was built and measured: 6.9 s a frame against 7.4–7.5 s
before (`zoom_frame`, minima of three, different loads) — nothing like the 97% it was aimed at, and why was not established (the 3 518 shadings sit in groups, each planned by its own
encoder, which is the first thing to check); so it was not kept. The lever is a chain shared by
content across those plans, or the scene stating one clip id per distinct path.

## 4. A frame's admission is its own: an atlas stale for the frame is reset and the frame encoded again

`issue1905.pdf` fits alone and was refused after `bug1721218_reduced.pdf` on the same device, on
HEAD's `raster-gpu` as on this one. The scratch sheet is not retained between frames (each encode
packs its own); what carries over is the **glyph atlas**. Logging every sheet reservation of the page
alone and after the other: after it, glyph tiles of 331 down to 179 pixels that a fresh atlas holds
went to the sheet instead, because the room probe (ADR 0093, ADR 1407) and the insert found the atlas
full of the earlier page's entries. The sheet's shelf packing then reached 12236 × 10798 where alone it
reached 11233 × 10825, and the frame was refused. A probe's refusal also never set `atlas_pressure`, so
the after-frame repack (ADR 0050) did not fire either. A cache decided a verdict.

**Built (`Device::encode_scene`, `AtlasStore::stale_for_frame`).** The atlas keeps a per-frame account:
whether the frame was refused room (by the probe or by an insert) and how many distinct keys it
used. After the encode, drawn or refused, if it was refused room while the atlas holds entries it did
not use, the atlas is reset and the frame encoded once more: on an empty atlas it is the frame a fresh
device draws, lanes, sheet, verdict and bytes. A frame whose own keys outgrow the atlas uses every
entry it holds and is not re-encoded (ADR 0050's condition for keeping what fits), so nothing loops.
The reset is reported as `atlas_repacked` on that frame. Cost: one extra encode on the first frame
after a change of working set that runs out of room, nothing otherwise.

**Test** (`tests/frame_independence.rs`, `a_page_after_one_that_filled_the_atlas_is_admitted_as_it_is_alone`):
a page of 40 glyph outlines over a large backdrop, at the least frame budget it is drawn under alone
(bisected), drawn after a page of 52 other outlines that nearly fills a 160 × 160 atlas — the same
verdict and the same bytes as alone. With the re-encode switched off it fails: "frame needs 79786
scene-derived bytes, over the stated budget of 79732". Two tests that stated the old dependence as the
behaviour (`atlas_budget.rs`'s second case, `encode_threads.rs`'s page after a refused frame) now state
that the frame repairs itself and says so, the second also holding that page to a fresh device's bytes
and counters; `retained_atlas.rs`'s "repacks once and then settles" holds unchanged, its one repack now
taken inside the first frame's encode.
