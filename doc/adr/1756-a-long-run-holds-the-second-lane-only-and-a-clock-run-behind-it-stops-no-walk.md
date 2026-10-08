# 1756 — A long run holds the second lane only, and a clock run behind it stops no walk

Session 1460. Status: **accepted** and **built**. Amends ADR 1684 section 1 (a fourth kind beside
small, large and clock, and a clock run's wait behind it) and the instruction ADR 1710 gave a seed
census. Supersedes nothing.
Context: ADRs 1684, 1706, 1710, 1716, 1746; traps 13, 25, 130.
Code: `tools/bounded.sh` (`--long`, `long_holds_lane2`, the clock turn in `lock_take`, self-test cases
11 and 12), `tools/state.sh` (`walk long`, `section_fuzz_stale`, `lock_cost`'s long holds),
`tools/main-checkout.py` (the re-seed instruction), `fuzz/seeds.sh`'s header, `doc/verify.md`'s
campaign lines, `doc/environment.md`'s rule line.
Tests: `tools/conformance/tests/bounded.rs` (`every_campaign_and_seed_census_holds_the_second_lane_only`,
`the_lock_cost_lists_each_long_hold_and_what_queued_behind_it`, the self-test, and the kind readers
that now know `--long`). Instrument: `scratchpad/r1460/replay/` (`table71.py`, `long71.py`, and ADR
1706's `replay.py` taught the long kind and the turn), kept at `/home/AI/lock-replay-1460/`.

## 1. The hole

A small walk takes the second lane, or the first when the second is busy, whatever its length. In
batch seventy-one round 1455 ran its census and campaigns as small walks and one `--tree 12` hold of
three campaigns: from 14:27 to 15:23 they held both lanes for all but 173 s, and round 1452's 25.4 s
corpus gate queued **4 289.1 s** behind `round=1455_fuzz/seeds.sh_check…` and `round=1455_…/large-hold.sh`
(`/home/AI/heavy-walk.log`). A campaign's length is its own choice (`-max_total_time`, a fork run of
an hour) and a census's is its population's, so neither belongs on a lane a walk might need.

## 2. The rule

- **`--long` declares a run whose length is its own choice**: a fuzz campaign, the `-runs=0` pass
  after one, a seed census. It takes the second lane and **only** the second: a second long run
  queues behind the first with the first lane free, and every walk keeps the first. Its `--tree`
  defaults to and may not exceed the second lane's 6 GiB (exit 64). `--long` without `--lock`, beside
  `--clock`, inside a hold of the first lane, or under a bare `flock` of it is refused (exit 64), with
  the cure: the outermost run declares it. The line says `kind=long lane=2`.
- **A clock run behind a long run stops no walk.** ADR 1684's gate stops every grant while a clock
  run waits, which is right for holds of minutes and wrong for one of an hour. So a clock run first
  takes the **clock turn**, `<lock>.clock`, which orders the clock runs and stops every long run's
  grant; while `<lock>.lane2.long` names a live long run's wrapper, it waits holding the turn and not
  the gate, and the first lane keeps granting walks; then it takes the gate and both lanes as before.
- **The exception is by peak, not by length.** `jbig2`'s and `jpx`'s census peaked at 5.34 to
  5.55 GiB (ADR 1710), inside half a gibibyte of the second lane's kill, so a census of those two
  alone stays a large walk on the first lane (270.7 and 591.2 s in batch sixty-seven). A campaign has
  no exception: round 1455's three in one hold peaked at 5.63 GiB and fit.
- `tools/state.sh gates-cost` lists each long hold with the runs whose `behind=` names its holder word
  and whose ask fell inside it, and sums their queue once each; `fuzz-stale` walks `long` and `large`.

## 3. What the rule gives on batch seventy-one's lines

ADR 1706's replay, each round's walks at their observed asks and holds; "long" relabels nine lines
(every campaign, census and pass after one; 14 170.8 s of hold). Calibration: the built rule's replay
gives the nineteen other walks 23 846.5 s against 23 053.5 s observed (+3.4%); the per-line misses are
the pollers' race ADR 1706 section 1 describes.

| rule | 19 walks' queue | median | max | over 1 800 s | 3 clock runs | 9 long runs | 1452's gate |
|---|---|---|---|---|---|---|---|
| built (ADR 1684) | 23 846.5 s | 1 169.8 s | 3 238.5 s | 8 | 596.4 s | 10 312.9 s | 726.1 s |
| `--long` alone | 21 217.8 s | 0.0 s | 5 622.2 s | 4 | 4 639.8 s | 30 542.1 s | 0.0 s |
| **`--long` and the turn** | **7 646.3 s** | 9.8 s | 1 380.0 s | **0** | 4 655.9 s | 30 606.5 s | 0.0 s |

**`--long` alone is not enough**, and that premise of the brief did not hold: pinned, the long runs
fill the second lane back to back, a clock run waits at the gate for each to end, and the gate stops
the first lane meanwhile — round 1452's `raster_golden` and 1451's drive each queue over 5 500 s. With
the turn the walks' queue falls by 16 200 s and none waits half an hour. Chaining on grants as well
gives 7 646.8 s, the same.

## 4. What it costs

- **A fuzz round has one lane.** Its long runs queue for each other, 30 606.5 s against 10 312.9 s in
  the replay, because round 1455's nine holds ran two at a time and now run one. The replay keeps
  every hold as observed; a round under this rule puts several targets in one `--long` hold under
  6 GiB, as `doc/verify.md` says, rather than one hold per target.
- **A clock run waits for the long run it found**, up to that run's end: 4 655.9 s against 596.4 s
  for round 1453's three. The turn stops the next long run from taking the lane, so the wait is one
  long hold, not a queue of them. The merge runs alone and is not affected.
- A long run that takes the lane in the half-second between a clock run's check and its turn is waited
  for behind the gate, as before this rule; the pollers' race is not ordered (ADR 1684 section 3).
- A wrapper killed with `SIGKILL` leaves `<lock>.lane2.long` behind; its pid is dead, which reads as
  no long hold.

## 5. Re-ask

After a batch that ran a `--long` hold, `tools/state.sh gates-cost` lists what queued behind each.
If a clock run's queue behind long holds exceeds the walks' queue the rule saved — `python3
table71.py <batch>` in the kept directory replays both — the turn is reopened, and the alternative
to price first is a long run that yields its lane at a campaign's restart point.

## 6. Taken with it, from the batch's other slots

- **A walk's `--build` is spelled as cargo takes it** (from slot 2, round 1457). The rule line said
  `--build '<profile> -p pdf-sandbox --bins'` and a round wrote `--build 'gates -p …'`, which
  `cargo build` refuses. The line now says `--build '--profile gates -p pdf-sandbox --bins'`, and
  `every_build_a_walk_declares_is_spelled_as_cargo_takes_it` holds every tracked instruction to a
  first word that is cargo's flag; `doc/todo/02` and the three trap files that spell trap 10's row
  are held, each another round's to re-spell.
- **`tools/state.sh batches` reads a grouped figure whole** (from slot 1, round 1456). The bodies have
  grouped digits in threes since batch sixty-four, and the reader took the last group: 2 135 s for
  batch seventy-one's 47 135 s, with its gate line unread. A one-to-three-digit number no digit
  precedes, followed by groups of three after a space or U+2009, is now joined first, and the gate
  line is read in its three shapes; `--batch-figures` is the reader, and
  `the_batches_section_reads_a_grouped_figure_whole` holds it to four bodies summed by hand.
