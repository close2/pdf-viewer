# 1541 — A meet's exact pixels are made beside the walk, and written where its tile was packed

Status: accepted. Session 1353. Answers ADR 1529 section 5, which named the chromatic render's
exact meet (0.80 G of its 0.99 G encode) and each render's pass recording (0.115 G) as what
stands between `bug1721218_reduced.pdf`'s zoom step and twice the CPU backend. Amends ADR 1395 in
one respect: on the GPU lane a residue-clipped mark leaves the walk's thread too. Supersedes
nothing.
Context: ISO 32000-2 §10.7.4, §8.5.4; ADRs 1395, 1467, 1503, 1513, 1517, 1529; traps 94, 105;
`doc/habits/measuring.md` 48, 63.
Code: `raster/crates/raster-gpu/src/encode/meet/deferred.rs` (new: `ExactMeet`, `ExactInputs`,
`Encoder::place_exact`, `settle_exact`, `finish_exact`), `encode/meet/helpers.rs` (new:
`Helpers`), `encode/meet.rs` (`Met`, `MarkInputs`, `Encoder::meet_residue`), `encode/coverage.rs`
(`coverage_tile`, `push_scratch_quad`), `encode/parallel/commit.rs` (`commit_sheet`,
`deferrable_bounds`), `encode/rare.rs`, `encode/scratch.rs` (`ScratchPacker::rewrite`),
`encode/meet/kept.rs` (`keep`, `keep_tile`), `encode/encoded.rs` (`finish`).
Tests: `raster/crates/raster-gpu/tests/exact_meets_settled_after_the_walk.rs` (new: four lanes and
thread counts at two budgets, every pixel to the closed form; watched failing with the settle's
write onto the sheet left out); `meet/kept.rs` `a_key_kept_twice_is_charged_once` (watched failing
without each of the two guards).

**`crates/render-cpu/` was not opened.** Its figures were taken through `zoom_frame`'s
`ZOOM_FRAME_BACKEND=cpu` (ADR 1529).

## 1. What the 0.80 G buys (callgrind by function, the 1.25× frame, one thread, GPU lane)

Re-taken on HEAD first: the chromatic render 1.148 G, `meet_residue` 0.804 G, of it `exact_areas`
0.474 G, `residue_intersection` 0.230 G, `residue_edges` 0.095 G; `draw_encoded` 0.115 G in each
render. Counted per pixel (habit 63) with the steps marked `#[inline(never)]` in a probe tree:
3 256 meets ask 11 988 pixels, each with 2 sets, 17.8 edges through it (8.4 the mark's, 9.4 the
clip's), 22.7 cuts and 38.4 partial edges beside it — 34.5 k instructions a pixel, spread over
`band_area` 0.112 G, `partial_windings` 0.088 G, `sort_row` 0.067 G, `sort_cuts` 0.059 G,
`crossings` 0.056 G and the mark's `RowEdges` 0.125 G. The marks are the page's dots, ellipses a
pixel and a half across, flattened to 32 points, under a clip of the same dots: every input of the
band machinery is already small, and no step is a third of it. 9 889 of the 11 988 answers are
below `min`, so the work is not idle. **A per-pixel lever would shave a step that is a sixth of
the whole; the whole is serial.** Skipping the exact pixels outright (a probe arm, wrong bytes)
took the step from 145.7 to 107.0 ms: 39 ms of the walk's thread.

## 2. Kept: the exact pixels leave the walk's thread

What a meet's exact pixels read is fixed when the walk reaches the mark: the tile as `min` left
it, the cut pixels, the chain's edges, the mark's polylines, rule and edges. Nothing later changes
them, and nothing later reads what they decide: the tile is packed onto the sheet and drawn from
it, and the device reads the sheet after `finish`. So the meet hands back an `ExactMeet` beside
the `min` tile; its caller packs the tile and records the meet at that place
(`Encoder::place_exact`). Helper threads, started once a frame's recorded meets reach 64 cut
pixels where the host allowed more than one thread, make the areas as the meets are recorded; the
settle at `finish` makes what none has claimed, waits for the rest, writes each finished tile over
its place on the sheet and keeps it for the next render, in encounter order. The helpers are let
go before the frame returns. A meet that settles is kept under its own words and its tile's
(ADRs 1517, 1529); a meet asked again before its first asking settled is computed again and the
second keep charges nothing. **A fan-out job's polylines and edges are taken, not copied**
(`MarkInputs::Owned`): a job's edges over its whole tile reach four times the tile's bound, and
copied for each of the stroked Type 3 page's meets they took its `turn_path` turn from 12.3 to
13.3 ms, past its 13.2 band; taken, it reads 12.05, HEAD's.

**On the GPU lane a residue-clipped fill or stroke now takes the fan-out's job too.** That lane's
reason for keeping marks on the walk (`take_gpu_lane`'s second question, ADR 0029) is answered no
on sight under a residue, so the mark was always the processor's; its meet then runs at the
fan-out job's commit, with the mark's edges built beside the fill (ADR 1513).

**Byte-identical by construction**: the same function of the same inputs over the same `min`,
whichever thread made it. What the recorded meets hold is counted against the queue's limit and
settled past it, the chain's kept edges counted once (they are the residue cache's, 4.98 MB on
this page; counted per meet they settled every meet alone).

**Tried and not kept.** All areas made at `finish` on a scope of the frame's threads: the settle
took 11 to 17 ms of eight pinned threads, the walk waiting. In one sitting the step read 113.5 to
115.9 that way against the helpers' 113.0 to 116.7, and the 1× frame 123.0 to 124.4 against 119.2
to 120.2, so the helpers are kept. The GPU-lane deferral read 111.0 to 117.7 in the same sitting,
about 2 ms, and is kept because it is what every other lane already did. The pass recording
(0.115 G a render, about 9 ms) was not taken: it is device calls per op, and the helpers were
the larger lever.

## 3. Measured

Pinned to the eight fastest cores, minima of 3 × 5 interleaved, exported HEAD and change trees in
their own target directories (`md5sum`-distinct), load 1.7, device busy under 10% before each:

| `bug1721218_reduced.pdf`, GPU lane | HEAD | change | the CPU backend |
|---|---:|---:|---:|
| `zoom_frame`, the 1× frame | 146.5 ms | 123.7 ms | 47.9 ms |
| `zoom_frame`, the 1.25× step | 135.0 ms | 117.7 ms | 57.4 ms |

An earlier sitting, before the GPU-lane deferral, read the step 135.4 → 112.1 against the CPU
backend's 56.4, 1.99×. **The step is 2.05× the CPU backend in the final sitting**: at 2×, not
inside it. At two threads the helpers cannot keep up (settle 15 to 19 ms, step 129.7). The table's
`frame_budget` rows, interleaved the same evening: turn 215.27 → 188.79 ms, step 134.12 → 117.04.

**Corpus.** `render-raster --test corpus` on the CPU, GPU and compute lanes at 1× and 4×, per-page
digests (`PDFVIEWER_RASTER_TIMES`) compared by name: **0 pages moved on any of the six arms**.
Verdicts equal HEAD's: at 1× CPU and compute 968 / 0 / 0, GPU 967 / 1 / 0; at 4× CPU and compute
964 / 0 / 3, GPU 963 / 0 / 4. One-versus-many 0 on every run.

## 4. What is left

The walk's thread still carries `residue_intersection` (0.23 G a chromatic render, the 3 515 small
chains' first fills) and `residue_edges` (0.095 G), each render's pass recording (0.115 G), and
the black render's walk. The first two decide later admissions in encounter order (ADR 1491), so
they are the walk's by construction; a chain's region made ahead of its first ask is the next
lever, the pass recording after it.
