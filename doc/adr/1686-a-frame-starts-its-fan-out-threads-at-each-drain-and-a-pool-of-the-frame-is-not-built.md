# 1686 — A frame starts its fan-out threads at each drain, and a pool of the frame's own is not built

Session 1422. Status: **accepted**; the instrument is built, the pool is not. Answers ADR 1668
section 5's second lever. Supersedes nothing; keeps ADR 0023's shape (threads entered inside
`Device::render` and left before it returns) and ADR 1505's claiming.
Context: `CLAUDE.md` principle 2 ("'genuinely' is decided by measurement") and the rule that an
optimisation is justified by a benchmark; ADRs 0023, 1505, 1519, 1541, 1567; traps 101 and 110;
habit 59.
Code: `raster/crates/raster-gpu/src/threads.rs` (`raster_gpu::threads::started`, a process's count
of the threads raster has started, counted at its four spawn sites: the encode's fan-out, the exact
meet's helpers, the image reduction and the ramp tables); the `threads` column of
`examples/frame_budget` and `examples/first_frame`, and `turn_threads`, `seventh_threads` and
`step_threads` on `tests/turn_path.rs`'s child line.

Every figure is the 890M through RADV, `examples/frame_budget` built `--release` from an export of
HEAD with this change, five rounds a run and the minimum of each, arms interleaved run by run behind
the lock, load 1.8 to 8.8. "Unpinned" is the 24 processors and 24 encode threads a viewer gets here
(`render_raster::options`); "pinned" is `turn_path`'s eight CPUs, so eight threads. Milliseconds.

## 1. How many, and where

| page, row | unpinned | pinned | which fan-outs |
|---|---|---|---|
| ISO 32000-2 p101, text — turn, seventh | 115 | 35 | five drains past the floor |
| `personwithdog.pdf`, vector — every row | 0 | 0 | no drain reaches the floor |
| `issue12841_reduced.pdf`, image — turn, seventh | 23 | 7 | not the encode's: they stay with its fan-out gone |
| every page's step | 0 | 0 | the compute lane queues nothing |
| `issue14415.pdf`, strokes — every row | 23 | 7 | one drain |
| `ContentStreamNoCycleType3insideType3.pdf` — every row | 46 | 14 | one drain and the exact meet's helpers |

On `examples/first_frame`'s page 7, frame 1 starts 46 unpinned (ADR 1668's figure, two drains) and
14 pinned, 49 at 4×, and frames 2 to 10 start none: a frame starts threads only for geometry it has
not seen.

## 2. The bound first: no thread started at all

A process-wide `rayon` pool in place of the per-drain scope, which no host here may have (ADR 0023:
oversubscription, the confined worker's seccomp filter, a pool built on the launch path) and which is
therefore an upper bound, never a candidate. Text page, encode, six runs: unpinned turn 4.53–4.83
against 4.05–4.26 and seventh 4.98–5.23 against 4.17–4.58, outside both spreads; pinned turn 4.68–5.13
against 4.48–5.00, inside. The stroke pages, the image page and the vector page move inside their
spreads. So the whole prize is about 0.5 ms of one page's encode, and only at 24 threads.

## 3. The candidate: one pool a frame, started at its first drain past the floor

Built and measured, then reverted: the encode walked inside a `std::thread::scope` of its own, its
fan-out threads started at the first drain that passed the floor and waiting on a condition variable
between drains, so the text page started 23 threads rather than 115 (7 rather than 35). The first
version passed `raster-gpu`'s 706 tests, `tests/encode_threads.rs`'s byte equality across thread
counts among them, and a test of three drains through one pool.

- **First version**: a thread handing its hold back woke every waiting thread, the threads were
  started before their batch was offered, and the claim order was asked of drains that divide
  nothing. Five runs: `issue14415.pdf`'s step 5.38–6.42 against 4.60–5.51 and the Type 3 page's step
  11.42–13.51 against 10.89–11.94, a loss on pages that drain once.
- **Last version**: the walk woken alone when the last hold comes back, the batch offered before the
  threads start, the order asked only of a drain that divides. Text page unpinned, six runs: turn
  4.16–4.54 against 4.53–4.83, seventh 4.33–4.83 against 4.98–5.23; eight runs more, turn 4.15–4.98
  against 4.28–4.88, seventh 4.37–4.73 against 4.52–5.45. Pinned, turn 4.36–5.19 against 4.68–5.13.
  `issue14415.pdf` inside HEAD's spread on every row; the Type 3 page's medians within 0.4 of HEAD's
  either way, its seventh above in both sets.

## 4. Decision

Not built. Starting 92 fewer threads moves the text page's encode by 0.1 to 0.4 ms at the median,
outside HEAD's spread in two of the six row-runs the last two versions were measured in; pinned, as
the gate measures, by nothing; and no other class, because their frames start threads once already.
The bound's other half is not this pool's to take: `rayon`'s threads go on running for a while after
their work ends, so they meet the next drain awake, where a thread woken from a condition variable is
a cold core's wake-up again — the reading consistent with trap 101, and not measured further here.
Keeping cores awake between drains is work done to raise a clock, which ADR 1519 declined. And the
pool puts the whole walk inside a scope, with a hand-off protocol of its own threaded through it, for
a figure the benchmark does not hold.

## 5. What would reopen it

A frame that drains many more times past the floor than five, or a host with more processors than 24,
read by the `threads` column; or a page whose exact-meet helpers and fan-out could share one set of
threads (the Type 3 page starts 46 where one set would do). The instrument says which in one run.
