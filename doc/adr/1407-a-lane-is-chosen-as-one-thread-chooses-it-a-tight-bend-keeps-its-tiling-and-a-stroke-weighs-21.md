# 1407 — A lane is chosen as one thread chooses it, a tight bend keeps its tiling, and a stroke weighs 21

Status: accepted. Session 1285. Amends raster's ADR 0093 (the room probe is answered from the
atlas a one-threaded walk reads) and ADR 0090's claim that the hybrid is zero pixels from the CPU
lane; keeps ADR 1375's construction against ADR 1397 section 1's largest lever left; re-takes
ADR 1395 section 3's `STROKE_SEGMENT_WEIGHT`. Supersedes nothing.
Context: ISO 32000-2 §8.4.3.2, §8.5.3.3, §10.7.4; raster's ADR 0054 (a frame's bytes do not
depend on its thread count); traps 54, 56, 58, 62, 65, 66, 67; habits 45, 52, 54; `doc/questions/A76`.
Code: `raster/crates/raster-gpu/src/atlas.rs` (`PendingInserts`, `probe_settled`),
`encode/parallel/commit.rs` (`prospect_for`, `probe_unsettled`, `note_atlas_insert`, the drain's
reset), `encode.rs` (`queued_inserts`), `encode/fill.rs` (the glyph lane records its insert),
`encode/parallel.rs` (`STROKE_SEGMENT_WEIGHT`), `raster/stroke.rs` (the comment at the tiling).
Tests: `crates/render-raster/tests/corpus.rs` (`OneThread`: every page drawn again at one encode
thread and held byte-equal), `atlas.rs`'s `a_settled_probe_answers_as_it_will_after_the_pending_inserts`,
`tests/compute_lane.rs`'s `the_compute_lane_is_within_one_level_of_the_cpu_lane_on_glyphs`.

**I did not open `crates/render-cpu/`.** Every expected value is the clause's or a measurement of
raster against itself; the oracle appears only as the corpus gate's verdicts.

## 1. Why a frame depended on its thread count

The brief's premise was that ADR 0090's hybrid and the scratch lane are not pixel-identical and
that this is the whole of it. Measured on `issue1905.pdf` alone, it is mostly not. With the room
probe on and the hybrid switched off, one thread and twenty-four differ by more (5 907 bytes, 11
levels) than with it on (999 bytes, 10 levels); at one thread, hybrid against scratch lane is 22
bytes, **one level**. The ten levels are ADR 0009's split: a tile the atlas admits is rasterised at
the **quantised phase**, a tile the probe refuses at its own transform. The probe reads the atlas's
shelves, and a job still queued has not inserted yet, so on many threads the probe admitted tiles
that one thread, having committed every insert before them, refused. 1286's repeat-key change
(ADR 1409) removes most of the drains that used to hide it: 53 corpus pages differed, not 13.

**The fix is the one-threaded answer, not a second lane.** Both lanes are §10.7.4's coverage of
the same set and neither is more correct, so the requirement is only that the choice be a function
of the bytes and the view. `probe_settled` says when no queued insert can change the probe — a
tile the atlas never admits, a resident key, a tile with no room now (inserts only consume), room
on a fresh shelf below every row the pending inserts could open, or room on a shelf beyond every
pending insert that shelf could seat (one at least half its height) — each bounded by the hull's
tile plus one pixel, the most a quantised phase adds. Otherwise `prospect_for` drains first.
Carrying the fill to its commit instead (`Job::follows`) was built and measured: carried fills add
no weight, the queue behind them never empties, and 123 536 fills went unsettled against 551
drains, `issue12295.pdf` 22× slower. Draining, twelve atlas-pressure pages, minimum of four
alternated runs at load 1.3: 366 ms against 333 unfixed (`issue12295` +24%, `pr12564` +29%, the
rest within 10%). The atlas test plants the rows and the shelf filter away and fails on each.

**The hybrid's one level** is real and small: two glyphs of that page (a `v` rotated 60°, a `w` on
a half-pixel baseline, coverage exactly one half: 127 against 128) at 300 placements read 6 pixels
on RADV and 2 on llvmpipe, one level each; the mosaic, all identity transforms, reads 0. The WGSL is
the CPU's arithmetic statement for statement; which operation rounds differently was not isolated.
Held to one level, ADR 0082's tolerance. `compute_lane.rs`'s own `device_with` leaves
`compute_assist` at `None`, so on RADV its CPU arm sends atlas-refused fills to the hybrid.

**The instrument** (trap 66's, made permanent): the corpus gate draws every page a second time at
one encode thread and holds the bytes equal. Corpus 1×: 959/2/6/7, **0 of 957 pages differ**.

## 2. The tiling stays (ADR 1375 against ADR 1389)

Tiling off, raster-gpu's per-pixel fixtures pass and only `the_pieces_of_a_tight_bend_tile_its_set`
fails; the hook at 16 w and the tight L draw to the byte as tiled at 1×–8× (27–152 pieces, no bound
reached). The corpus, every first page both ways: 24 pages move, **19 by more than a sixteenth**, all
darker untiled. `bug1743245.pdf`: 205 tight-bend strokes, 314 pieces on average and 1 273 at most,
and every one of them spends the fill's sweep (`TESTS_PER_UNIT`) — 7 173 bytes past 16 levels, up to
184. `issue14415.pdf`: 403 such strokes, 321 sweeps spent, up to 74. The Type 3 page is the one where
tiling is redundant (4 bytes, 2 levels) and costs most (58 → 20 ms). So §8.4.3.2's set is painted
exactly by the fill alone only within its bounds, and a 950-piece stroke is outside them. Tiling
only where the bound would be hit needs the consumer to fill untiled, detect the bound and retile —
in the encoder's `rasterise` and the walk's stroke path — and the GPU triangle lane integrates
without the set at all, so it would keep the tiling anyway. Not built; the pieces-tile test stands,
since the construction it states is the one kept.

## 3. `STROKE_SEGMENT_WEIGHT` is 21

ADR 1395's method on this tree: every job of the corpus's first pages at 1×, one encode thread,
`taskset -c 2` (a performance core), timed in `rasterise`, two runs: 166 726 glyph fills 0.1375 /
0.1378 µs a segment, 57 118 strokes 3.011 / 3.014 — **21.9**, rounded down to 21. (All 172 762 fills
with the 6 036 sheet fills: 12.8.) ADR 1397 made a stroke 13% cheaper.

## 4. Left

**At 4× on the CPU lane (the page-turn lane) pages still differ between one thread and many**:
176 of 966 with this fix, 375 with the settled-probe check off, up to 220 levels on one page, most
under 15. Four of them drawn alone (`issue13003`, `issue1293r`, `issue14046`, `issue12963`) agree, so
the divergence is carried from page to page by the rasteriser's retained atlas: a frame-by-frame
trace of `settle_atlas` shows the two arms' first difference as one frame's atlas pressure, 74 against
75 entries used and 5 297 against 5 371 resident, after which the repack decisions part. Which insert
first differs was not isolated. The corpus gate therefore holds the list empty at 1× and prints it
as a survey at other scales. Asking `winds_two_values` of the identity flattening always was tried
and moves nothing at 4× (176 again), so the `OnceLock` below is not this residue.

`StoredOutline::winds_two_values_as` and `winds_two_values` share one `OnceLock`: the walk's
`compute_takes` asks it of the identity flattening and a worker of its placement's, and which arrives
first depends on the thread count. Placement-invariant by construction would be asking the identity
flattening always (`resources.rs`), at the price of reading a glyph's topology at one coarse
flattening; no corpus page shows it.
