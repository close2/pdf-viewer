# 1395 — A clipped mark's coverage is made off the walk's thread, and a stroke weighs what it costs

Status: accepted. Session 1279. Extends raster's encode seam (`encode/parallel.rs`, raster's ADR
0051 and `raster/doc/notes-encode-threads.md`); supersedes nothing; does not touch ADR 1375's or
ADR 1389's constructions, whose costs it takes off the walk's thread rather than out of the work.
Context: `CLAUDE.md` principle 2 (for a viewer, latency before throughput); ISO 32000-2
§8.5.4 for the clip the commit multiplies in.
Code: `raster/crates/raster-gpu/src/encode/parallel.rs` (`Draw::under`, `STROKE_SEGMENT_WEIGHT`,
`weight_of`), `encode/parallel/commit.rs` (`deferrable_bounds`, `commit_sheet`), `encode/coverage.rs`
(`multiply_residue`), `encode/clips.rs` (`residue_intersection` drains first), `encode/stroke.rs`,
`encode/fill.rs`.
Tests: `raster/crates/raster-gpu/tests/encode_threads.rs` (`a_clipped_run_meets_its_clip_at_every_thread_count`,
a clipped stroke in `busy_page`), `encode/parallel.rs`'s `a_stroke_weighs_its_segments_at_their_measured_cost`.

## 1. What five batches of rasteriser work cost a page turn

`examples/frame_budget`, the 890M through RADV, 1600 × 1000, minimum of five rounds, pinned to the
performance cores, one sitting, load 3.0–3.6, one build per commit in its own target directory,
binaries `md5sum`-distinct. Turn row, milliseconds, budget / encode:

| page | before ADR 1341 | 1341 | 1348 | 1359 | 1373–1375 | 1389 |
|---|---|---|---|---|---|---|
| ISO 32000-2 p101 | 9.06 / 6.27 | 9.22 / 6.40 | 9.23 / 6.30 | 9.20 / 6.37 | 9.01 / 6.30 | 10.31 / 7.47 |
| `issue14415.pdf` | 10.28 / 5.07 | 9.71 / 4.85 | 9.66 / 4.85 | 10.38 / 5.54 | 23.28 / 18.46 | 40.49 / 35.56 |
| `issue19802.pdf` | 3.00 / 1.37 | 2.93 / 1.37 | 2.93 / 1.36 | 2.95 / 1.37 | 2.94 / 1.37 | 7.23 / 5.65 |
| `ContentStreamNoCycleType3insideType3.pdf` | 4.64 / 3.33 | 4.61 / 3.29 | 4.60 / 3.27 | 4.74 / 3.44 | 40.23 / 38.87 | 47.17 / 45.59 |

`22060_A1_01_Plans.pdf` (139–147) and `images.pdf` (150–156) are interpretation and transfer and
did not move. Two constructions carry every move: ADR 1375's tiling of a tight bend (the Type 3
page ×8.7, `issue14415` ×2.3) and ADR 1389's fill set (+19% encode on the text page, ×4.1 on
`issue19802`). ADRs 1341, 1348 and 1359 move none of these rows; their walk is `render-cpu`'s.

## 2. Why the Type 3 page drew on one thread

72 of its 85 strokes carry a residue clip (a probe of `encode_stroke`, one frame), and
`deferrable_bounds` kept every such mark on the walk's thread because the residue is read out of
a cache the frame decides about in encounter order (`encode/residue.rs`). The comment above it
called that a lane not yet taught the seam rather than a limit of it, and it was.

**The construction.** The job makes the mark's coverage over the tile the walk would have made —
sized by the chain's `mark_bounds`, folded with the target by the same `max`/`min` identity
`deferrable_bounds` already argues — and the draw carries the resolved chain to the commit, which
charges the tile and then multiplies the residue in (`multiply_residue`, now the one site for the
walk's tiles and the fan-out's). Every call of `residue_intersection` drains the queue first, so
the first tile to ask about a chain, which decides whether its region is kept, is the one a
one-threaded walk would have asked with. A clipped fill does not take its outline's two-values
answer (ADR 1389): the walk asked the tile's own question, and so does the job.

## 3. Why `issue14415.pdf` drew on one thread

Every drain of its turn was under `PARALLEL_FLOOR_SEGMENTS` (4 096 segments): 194 stroke jobs
weighing 2 820 took 6.7 ms on the walk's thread. The floor counts outline segments, and a stroke's
segment is not a fill's. Every job of the pdf.js corpus's first pages at 1×, timed on one pinned
performance core: 189 083 glyph fills, 0.139 µs a segment; 56 621 strokes, 3.456 µs a segment;
ratio 24.9. `STROKE_SEGMENT_WEIGHT` is 24, rounded down so the floor is reached no earlier than
the costs say. It moves which thread runs a job and nothing it makes. The ratio is HEAD
2f571015's; a construction that makes a stroke cheaper lowers it, and the per-job timing (a probe
in `rasterise`, one pinned core, `PH_THREADS=1`) is how it is re-taken.

## 4. Numbers

Turn row, base / residue seam alone / both, pinned, load 1.4–2.7, budget / encode (ms):

| page | base | seam | both |
|---|---|---|---|
| Type 3 page | 46.73 / 45.20 | 16.38 / 14.90 | 16.92 / 15.51 |
| `issue14415.pdf` | 39.92 / 35.09 | 39.72 / 35.02 | 18.93 / 14.12 |
| ISO p101, `issue19802`, plans, images | unchanged within the run's spread | | |

Unpinned (what a person gets): Type 3 46.99 → 15.77, `issue14415` 40.08 → 20.11. The step rows
(compute lane) do not move: that lane does not fan out.

Callgrind, one round of each page: the Type 3 page 2 100.3 M → 2 101.0 M instructions,
`stroke_polylines` 1 138.7 M → 1 140.6 M, moved from `Encoder::command` to `parallel::rasterise`;
`issue14415` 1 483.5 M → 1 498.2 M (+1.0%, the scope's thread spawns), every raster function
within 1%. The work is the same; it moved off the walk's thread.

**Exactness.** Every pdf.js first page hashed through raster's CPU lane, base against this change:
957 pages identical at 1× on 24 threads, at 1× on one, at 2× and at 4×. The new test fails when
the commit's multiply is removed (planted and restored).

## 5. Left, with the measurement

- **A text page's turn is still serial.** ISO p101: 85 drains forced by `prospect_for`'s
  repeated-key guard carry 82.5 M instructions and 251 forced by a rectangle instance 39.5 M
  (callgrind, turn and warm-up); no drain reaches the floor. Queuing a rectangle instance as a
  job with no geometry, committed in order, is the next lever.
- **The compute lane's step** on the Type 3 page (54 ms) expands every stroke on the walk's thread.
- **A frame's bytes depend on its thread count on 13 corpus pages, before this change.**
  `issue1905.pdf` alone on a fresh device: one thread differs from two by 237 bytes, up to 10
  levels, in x 703–845, y 387–406; with `prospect_for`'s room probe switched off, the whole corpus
  is identical at 1 and 24 threads. The probe's own comment says a queued insert can make it
  admit a tile the one-threaded walk reroutes to ADR 0090's hybrid, and counts on the two lanes
  agreeing to the pixel — so the hybrid and the scratch lane are not zero pixels apart there.
- `StoredOutline::winds_two_values_as` answers from whichever worker arrives first; the answer is
  placement-invariant only in exact arithmetic. No corpus page shows it (the walk above).
- ADR 1375's per-placement cache: a translated placement flattens to other floats, so a cache
  keyed on anything wider than the exact transform is not the same bytes.
