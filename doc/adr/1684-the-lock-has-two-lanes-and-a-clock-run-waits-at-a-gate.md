# 1684 — The lock has two lanes, and a clock run waits at a gate

Session 1425. Status: **accepted** and **built**. Builds what ADR 1671 section 4 decided was worth
building; amends ADR 1646 section 1 ("one heavy walk at a time" becomes one per lane) and the line
shape of ADR 1659 section 3. Supersedes nothing.
Context: ADRs 1646, 1659, 1671, 1674; traps 13, 25, 110, 131.
Code: `tools/bounded.sh` (`lock_take`, `lane_hold`, `lock_queued`, `lanes_open_in`,
`marked_ancestor_lanes`, `lock_record`, `without_lanes`, `--clock`, self-test case 9),
`tools/batch.sh` (`clock_gates`, `run`, the arms line's comment), `tools/state.sh` (`lock_cost`'s
lane column), `doc/environment.md`'s rule line, `doc/todo/02` section 0 item 3.
Tests: `tools/conformance/tests/bounded.rs` (`every_lock_a_caller_takes_declares_its_kind`,
`every_clock_gate_the_merge_names_is_a_gate_it_runs`, the self-test), `batch.rs` (the arms-held
test asks both lanes). Instrument: `scratchpad/r1425/replay/built.py` over 1417's `lock.py` and
`sim.py` (kept at `/home/AI/lock-replay-1417/`), not kept.

## 1. The rule

- **Two lanes and a gate**: the first lane is the lock file, the second `<lock>.lane2`, and
  `<lock>.gate` is the order. A run's **kind is what it declared**, because the lane is granted
  before the walk starts: `--clock` is a clock run, a `--tree` of 6 GiB or less a small walk, any
  other a large one. A small walk takes either lane, the second first, so the first stays free for a
  large one; a large walk takes the first; a clock run takes both. Two lanes are at most 12 + 6 GiB
  of walks, which ADR 1659 section 2 found room for beside everything else, and only because
  `--tree` kills.
- **A waiting clock run stops every grant**, the walks queued before it included. It takes the gate
  and keeps it while it waits for the first lane and then the second, and a walk is granted a lane
  only at a moment it finds the gate free. `flock` waits for one file, so a walk polls the gate and
  its lanes twice a second; a clock run blocks in the kernel, behind the gate. Every taker takes the
  first lane before the second, so no two each hold the lane the other waits for.
- **A `--clock` inside a hold of one lane is refused** (exit 64) and names the cure: the outermost
  wrapper declares `--clock`. Taking the missing lane would mean holding the second while waiting
  for the first. Under a bare `flock` of the first lane, a clock run takes the second after it.
  `--clock` without `--lock` is refused.
- **The line names both**: `… behind=<holder> kind=<large|small|clock> lane=<1|2|1+2> cmd=…`,
  between `behind=` and `cmd=`, so `lock_cost`'s fields one to six and its `cmd=` stay where they
  are. `behind=` joins the holders of every lane the run waited for with `+`, or names the gate's
  clock run. A line with no `lane=` was written on one lane.
- **What is a clock run**: one whose verdict a neighbour's load can move. That means a band or
  floor on a time, an A/B, or a reference program under a budget (`doc/todo/02` section 2, "Run the
  sequence on a quiet machine"). The merge's are named once in `clock_gates`: the transform gate's
  floor, `turn_path`, the oracle, text extraction, the quorra corpus gate and `foreign_corpus`.
  The merge runs alone, so the list costs it nothing. Every other gate is a large walk.
- **The arms export stays large.** Its two builds run inside the hold, and a cold build is not held
  to 6 GiB. A killed export costs the batch its pixels baseline, and the rounds' small walks run
  beside it on the second lane.

## 2. What the built rule gives on batch sixty-four's log

The replay is 1417's `sim.py` (each round's walks in its own order, each asked the observed gap
after the round's previous one ended, each held its observed time), stepped as events over the two
lanes. Clock runs are ADR 1671's: round 1411's A/B children and the 189 s of `turn_path` inside
1410's arms. Every other walk is declared by its observed peak, and all of batch sixty-four's were
under 6 GiB. The `sccache` hold is gone (ADR 1674) and the open's export holds the first lane.
**Calibrated** (trap 13) by running the stepper under the model's own rule — a waiting clock run
stops only later asks — where it reproduces `lanes.py`'s two rows exactly. Under one lane it gives
14 396.5 s against `sim.py`'s 14 363.3, 33.2 s apart, all of it round 1408's two walks in flight at
once.

| rule, five rounds' queue | queue | against one lane's 14 363.3 s |
|---|---|---|
| ADR 1671's model, ask order | 5 443.7 s | −8 919.6 |
| the model, 1410's whole arms a clock run | 7 978.6 s | −6 384.7 |
| **built: a waiting clock run stops every grant** | **4 788.6 s** | **−9 574.7** |
| built, only walks that peaked under 4 GiB declared small | 7 189.6 s | −7 173.7 |
| built, only walks that peaked under 2 GiB declared small | 9 653.5 s | −4 709.8 |

**The gate saves more than the model, not less**, for ADR 1671 section 2's reason. Batch
sixty-four's clock runs were its short holds (1.1 to 232.4 s), so letting a waiting one pass the
walks queued before it shortens the queue, as shortest-first did on one lane. **The saving is the
rounds' to keep by declaring**: a walk left at `--tree 12` is a large one, and the last two rows
are what under-declaring costs. These are best cases. Two walks side by side stretch each other's
holds, and the replay does not model it.

## 3. What stays, and the re-ask

- **No order among walks.** Pollers race for a freed lane, so a walk queued first is not
  necessarily granted first. Clock runs pass the gate in the kernel's order, and a stream of them
  could starve the walks; batch sixty-four had eleven.
- **A small walk is killed at 6 GiB**, a build inside it included. A round unsure of its peak
  declares 12 and pays the queue rather than the run.
- `tools/state.sh`'s corpus sections run their walks through `tools/bounded.sh` without `--lock`.
  A round that calls one runs it under its own `--lock` line, which declares the kind.
- **Re-ask**: ADR 1671's, unchanged and now answerable from the log alone. After the lanes' first
  batch, replay its lines under one lane and under the built rule, kinds from `kind=`. If the two
  differ by less than 3 600 s, the second lane comes out.
