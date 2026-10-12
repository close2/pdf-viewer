# 1882 — The strip planner reads the list on the pool and decides the cut on one thread

Session 1523. Status: **accepted** and **built**. The owner's performance target for batch
eighty-three; round 1515 left the planner as half a zoomed processor draw of the witness (ADR 1868
§1). Context: ADRs 0137, 0139, 0219, 0687, 1758, 1819, 1837, 1841, 1859. Code:
`crates/pdf-render/src/strips.rs` (`strip_inputs`, `StripInputs`, `Piece`, `OwnBound`),
`crates/pdf-render/src/reach.rs` (`MarkBounds::exactly_at`), `crates/render-cpu/src/lib.rs`
(`plan_strips`). Held by `pdf-render`'s `the_inputs_read_on_the_pool_are_the_serial_inputs` and
`raster_golden`. Instrument: `viewer-host`'s `examples/band_first`. No ledger row moves.

## 1. What the planner did

`plan_strips` asked four serial functions of the list before it cut a page: `command_extents`,
`row_costs`, `unsplittable_rows_and_freed_fills` and `replay_ratio`, the last once per candidate
division. Callgrind of one plan of the witness at 5.86 times the fit, 3569 × 2526, one thread:
5 964 M instructions. `Command::device_bounds` was 2 634 M of them (44 %), called twice per
command, once in each walk. `command_extents`' walk was 1 979 M, the forbidden-rows walk 2 092 M
(the backend's `drawn_row_by_row` 658 M of it), `replay_ratio` about 470 M. Then the draw measured
every bound a third time, on the pool, for `MarkBounds` (ADR 1841). So the brief's hypothesis held
in half: the planner's per-mark work is `MarkBounds`' work twice, and what is left is not a cheap
scan but two walks and a replay.

## 2. Decision

1. **The bounds are measured before the plan, and the plan reads them.** `plan_strips` asks
   `DisplayList::mark_bounds` first and the draw then finds them kept. A top-level command's bound
   is read only where they were measured under the target's own transform, all six numbers the same
   bits (`MarkBounds::exactly_at`); a scroll's moved bounds are not the function's answer, so the
   planner measures those again.
2. **The list is read on the pool in runs of commands** (`strip_inputs`): at least 4 096 a run, four
   runs a thread, fewer where two row vectors a run would pass 64 MiB. Each run gathers its own
   extents and marks its own forbidden and freed rows.
3. **Every number the cut decision reads is the serial one, bit for bit**, so the cuts are the
   same cuts. The extents are the runs' in order. A row's cost is summed in the list's order, each
   band of rows on its own thread; each run sorts its extents into the bands they reach. A cost
   summed per run and then joined differs in 668 of the test page's 818 rows. The forbidden rows
   are a union, the same in any order: `settled` only ever skips arithmetic (ADR 0687), and a run
   that cannot see an earlier run's rows does more of it, never less. The freed rows alone are not:
   a fill skipped inside rows already forbidden is marked free by a run that has not seen them.
   So only their union with the forbidden rows is stated, which is all the planner reads (ADR
   1837). The replay is two whole-number counts.
4. **The cut stays serial**: `strip_boundaries_avoiding` and the count search are unchanged, 5–6 ms
   on the witness. The serial functions stay public, for the instruments and as the test's
   reference.

## 3. Evidence

- **Exactness.** The equality test reads a 40 001-command page on pools of one, four and seven
  threads, with the bounds kept at the target and at another translation. Three plants each fail
  it: the bounds read one command out, costs summed per run, the freed rows dropped from the
  union. A fourth fails it too: a band boundary left out of the sorting. On the witness the cuts
  are HEAD's at 1×, 5.86× and 7.33×, at sixteen strips and at a pool of four and of twenty-four.
  `raster_golden` from HEAD plus these hunks: held 974, moved 0; the division test 176 pages at
  most 1 308 pixels and 3 levels, the brief's figures. Two of the six arms, cpu 1× and 4×, were run
  from the same export: 968 and 965 pages, 0 digests and 0 mean errors differ from
  `/home/AI/arms-1522/`. The other four are owed none, because their oracle is this planner's
  frame and it did not move.
- **The witness**, one clock hold, HEAD's export and HEAD plus these hunks alternated three times,
  `band_first` with a window of 609 × 1019, three rounds a zoom, load 4.5–6.6, ms:

  | pool | zoom | planner, HEAD → this | page drawn whole, HEAD → this | its rows shown first |
  |---|---|---|---|---|
  | 24 | 5.86 | 231–351 → **43–48** | 553–660 → **337–378** | 289–344 → 209–242 |
  | 24 | 7.33 | 232–236 → **39–43** | 630–712 → **378–452** | 279–328 → 218–314 |
  | 4 | 5.86 | 206–359 → **55–102** | 849–1 280 → **611–866** | 341–493 → 269–403 |
  | 4 | 7.33 | 210–217 → **55–72** | 993–1 048 → **720–1 015** | 369–508 → 291–399 |

  The planner is asked at the pool's width with the bounds kept, as the draw asks it. On
  twenty-four threads its median was 36–41 % of the median draw, and it is 10–13 % now. The first
  plan at sixteen strips, which measures the bounds itself, is 60–79 ms against HEAD's 233–340. No
  round's pages differ.
  `turn_path` in the same hold: 33 figures judged, 0 outside.

## 4. What it leaves

- By a scratch build's timers outside the hold, the reading is 22–29 ms on twenty-four threads and
  51–68 ms on four, the cost bands 9–28 ms and the search with its replays 5–6 ms. A band re-derives
  each extent's rows from its index; keeping the rows instead costs 16 more bytes per extent.
- The draw itself, about 290–410 ms of the twenty-four-thread figure, is the next term.
  `doc/todo/69` §7 names it for the first frame.
