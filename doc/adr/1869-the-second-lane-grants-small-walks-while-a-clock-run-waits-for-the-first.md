# 1869 — The second lane grants small walks while a clock run waits for the first, and the batch table reads the merge's gate line

Session 1516, an instruments round of batch eighty-two. Status: **accepted** and **built**. Amends
ADR 1684 section 1's second rule ("a waiting clock run stops every grant") and keeps ADR 1756's
clock turn and long lane and ADR 1790's ask order as they are. Answers review 1513's two instrument
findings (section 1 and section 1.1.1). Context: ADRs 1684, 1756, 1790, 1863; traps 11, 13, 130.
Code: `tools/bounded.sh` (`lock_take`, `queue_look`, `small_walk_overran`, `small_walk_ceiling`,
`first_path`, self-test cases 9 and 14), `tools/state.sh` (`batch_figures`, `lock_cost`).
Tests: `tools/conformance/tests/bounded.rs` (the self-test,
`the_lock_cost_sums_the_small_walks_queue_while_the_second_lane_stood_free`), `state_sections.rs`
(`the_batches_section_reads_a_grouped_figure_whole`). Lines: `doc/environment.md`'s rule block,
`doc/todo/02` section 0 item 3. Instrument: `sidelane.py`, kept at `/home/AI/lock-replay-1516/`.

## 1. What the gate cost

A clock run took `<lock>.gate` and kept it while it waited for the first lane and then the second,
and no walk was granted either lane while the gate was held. Behind a large walk of 1 000–1 700 s
the second lane therefore stood free while small walks queued for it. `doc/reviews/1513/lanes.py`
reads that idle time off the log as 5 365, 3 545 and **9 179 s** for batches seventy-nine to
eighty-one. Read the other way round — the seconds of a small walk's queue during which nothing held
the second lane, which is the figure `tools/state.sh gates-cost` now prints — the same batches read
9 035.6, 9 406.8 and **19 705.7 s**, and the longest single wait of that kind was 5 997.8 s.

## 2. The rule

- **The first lane is owed first, and the second only once the first is taken.** A clock run takes
  the clock turn as before, waits out a long run on the second lane as before, and then holds
  `<lock>.first`, the new file, while it waits for the first lane. No walk is granted the first lane
  while `first` is held, the ones queued before the clock run included. When the clock run has the
  first lane it takes the gate as well, and from then on no walk is granted either lane until it has
  the second.
- **While the clock run waits for the first lane, a small walk is still granted the second**, in
  ask order among the walks that take it. A large walk takes only the first and waits. A long run is
  stopped by the turn as before, and **its ticket holds back nobody while the turn is held**: it
  cannot be granted, and an earlier long ticket would otherwise keep the second lane idle behind the
  clock run.
- **No walk is granted the second lane after the clock run has the first.** The clock run looks
  for the first lane with `flock -n`, under the gate held for that look only. A walk looks and is
  granted under the gate held *shared*. The two never look at once. The price is the clock run's
  poll, up to half a second after the first lane frees, where the kernel used to hand it over at
  once.
- **A small walk is one that ends within 600 s** (`small_walk_ceiling`); a longer one declares
  `--long`. The ceiling is stated and not enforced, because a wrapper cannot know a walk's length
  before it ends. A small walk that held the second lane past it *and* finds a clock run holding the
  gate when it ends — the one condition in which its length cost a clock run (trap 11) — prints both
  figures and the cure. A small walk that ended before the clock run held the first lane says
  nothing.

## 3. What it gives on the last three batches

`sidelane.py` replays a batch's lines. Each line keeps its observed hold. Each is asked the observed
gap after its own round's latest earlier ask or release, so a walk granted earlier lets its round's
next walk ask earlier (ADR 1706's chain). A `lane=-` line asks and leaves at its observed wait.
**Calibrated** (trap 13): the built rule, granted in the observed order, puts 105 of 112 lines
within 1 s of the log for batch seventy-nine, 90 of 103 for eighty-one, and its idle time for
eighty-one is lanes.py's 9 179 s exactly. Batch eighty replays 59 of 93 within 1 s, max |d| 2 509 s,
because of its four stopped waits, so its row is weaker than the other two.

| batch | rule | rounds' walks queued | small walk, longest | clock runs queued | lane 2 idle behind the gate |
|---|---|---|---|---|---|
| 79 | built | 33 526 s | 1 974 s | 15 278 s | 5 368 s |
| 79 | **side lane** | **25 784 s** | 1 236 s | **10 644 s** | 4 385 s |
| 80 | built | 50 707 s | 4 496 s | 11 201 s | 1 148 s |
| 80 | **side lane** | **36 725 s** | 2 375 s | 15 645 s | 1 283 s |
| 81 | built | 54 413 s | 6 110 s | 42 815 s | 9 179 s |
| 81 | **side lane** | **26 245 s** | 6 062 s | **30 522 s** | 2 464 s |

- **The walks' queue falls by 23–52 %**, 7 742 to 28 168 s a batch.
- **The cost the rule is about is 84 s in three batches.** That is one clock run, in seventy-nine,
  which waited for the second lane behind a side lane walk once the first lane was its own. Clock
  runs' queues move by −4 634, +4 444 and −12 293 s in total, and nearly all of that movement is the
  rounds' own timelines moving, because walks ending earlier ask their next run earlier. It is not
  the side lane holding them.
- **Live, on this round's own clock hold** (the rule was in place from 13:27): it asked at
  13:53:28 and waited 125 s for the first lane, behind a small walk of round 1517's. Meanwhile three
  small walks were granted the second lane, 1515's twice (45 s each, one with no wait) and 1519's
  once (13 s), and the gate had stopped all three before. It then waited 13 s for the second lane
  behind the last of them, and it was granted both at 13:55:47.
- **The ceiling is 600 s and not 300.** Over the three batches 95 small walks held a lane for a
  median of 21 s, p90 200 s, p95 304 s and at most 1 672 s (round 1506's six-arm run, declared
  `--tree 6`). Replayed with every small walk over 600 s declared `--long`, the table above does not
  move. Over 300 s, batch eighty's clock runs queue 12 627 s more, because a long run is never
  granted while a clock run holds the turn and so waits out every one of them.

## 4. Pinned

Self-test case 9 is rebuilt around the rule. A large walk holds the first lane and a small walk the
second, and then a clock run asks. While the clock run waits: a large walk asked after it is not
granted the first lane before the clock run ends, though it looks five times as often; a long run
asked after it is not granted the second; and a small walk asked after both is granted the second
when it frees while the first is still held. Then the first frees, and a small walk asked once the
clock run has it is not granted the second lane before the clock run ends. Each assertion was
planted out on a copy and the case failed: with the gate held through the first lane's wait, "a small walk asked while a
clock run waited for the first lane was not granted the second when it freed"; with the gate let
go before the second lane, the late walk "never queued at the gate"; with the first lane's claim
ignored, the large walk was granted the lane before the clock run ended; with long tickets holding
back the walks behind them, the side walk was not granted; with the ceiling's sentence removed, "did
not say so". Three runs side by side passed. Case 14 checks that a clock run stopped while it
queues leaves `first` free.

## 5. The batch table

`tools/state.sh batches` read the gate line with `N of M green`, and from batch seventy-four the
bodies say `33 of 33 gates green, 1 836 s of gate wall time` (and in eighty-one `34 of 36 gates
green at the run, …`). So every batch since printed "no gate wall-time line". The pattern takes the
word `gates` as optional, and the section prints the gate figure for all eight. Two of those bodies
are in the section's test.

## 6. What it costs, and the re-ask

- A clock run takes the first lane up to a poll after it frees, once per hold.
- A small walk granted the second lane while a clock run waits for the first delays it by whatever
  of the walk is left when the first lane frees. The ceiling is what bounds that, and it holds only
  for as long as rounds declare by it.
- Until the merge, a wrapper from another checkout ignores `first` and can take the first lane ahead
  of a clock run waiting for it, as ADR 1790 said of tickets.
- **Re-ask**: after the next batch, `tools/state.sh gates-cost`'s line for the small walks' queue
  while the second lane stood free is a poll a walk, half a second, or the rule has failed; and if
  the clock runs' queue, read with lanes.py, rises past the built rule's replay of the same lines,
  the side lane is priced again against `sidelane.py`.
