# 1286 — A mark with no geometry queues, and a zoom step's strokes divide

Batch forty-four, performance slot. No ledger row moved: the contract named none. ADR 1409.

## Measured first (own exports and target dirs, `md5sum`-distinct; a drain probe in a copy only)
- p101 turn: 85 drains by the repeated-key guard (57 831 weight) and 91 by a rectangle (28 690),
  none at the floor. Either lever alone: 95 or 91 drains, turn unmoved (10.1–10.4 ms); both: one
  drain of 84 355 at the frame's end.
- The compute lane's step: `deferrable_bounds` kept every stroke on the walk's thread, and every
  compute tile drained (`issue14415.pdf`: 519 drains, none at the floor).

## Built (kept)
- A rectangle behind a queued mark is a job, written at its commit.
- A repeat of a queued atlas key is a job reading the entry at its commit, and walks the fill
  again there where the atlas lacks it; `drain_queue` loops for what that queues.
- Strokes and curve-clipped fills leave the thread on the compute lane; a compute tile behind a
  queued mark is a job, seated at its commit.
- Pinned turn/step, base → this (one sitting): p101 turn 9.89 → 6.78; `issue19802` 6.90 → 4.13;
  `issue14415` turn 18.34 → 12.13, step 21.31 → 6.06; Type 3 turn 15.89 → 11.23, step 54.00 → 12.65.
  Callgrind totals within +0.14% (p101), +0.06% (Type 3), −2.0% (`issue14415`: its tiled strokes
  stop asking on the compute lane); the walk's thread 153.8 → 51.4 M, 1 101 → 25.3, 569.5 → 32.0.

## Measured and dropped
The three-valued outline cache: −0.15% on the Type 3 page; the regions still asked are strokes'.

## Exactness
Corpus 1×: 959/2, thread count 0. 4× page-turn lane: 353 pages differ at 1 and N threads at HEAD,
354 with this change alone, 0 in both with the room probe off — the probe's population (ADR 1407's
subject); the worktree with 1285's fix in progress read 176. 4× compute 958/0 and sampled 957/0,
thread count 0 each. Plants: the walk-again, the rectangle commit's drain, the loop each fail
`a_text_page_is_the_same_bytes_at_every_thread_count`.

## Gates
raster-gpu nextest 616/616; render-raster 93/93; conformance 0; headless_gpu 39/39; launch_path
(clocks) 0, 42/42 in band. raster_golden exit 101: 7 moved, none reachable from raster-gpu (6
reports-only, `freetext_no_appearance.pdf` a sibling's interpreter change). Clippy: `fill_solid`
is 106 lines with 1285's in-progress lines (this change's net is −2).
