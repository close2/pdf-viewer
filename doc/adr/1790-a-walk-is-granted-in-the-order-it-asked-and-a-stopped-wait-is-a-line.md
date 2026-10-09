# 1790 — A walk is granted in the order it asked, and a wait stopped before its grant is a line

Session 1477. Status: **accepted** and **built**. Amends ADR 1684 section 3 ("No order among walks")
and the line rule of ADR 1646 (one line per *hold*). Leaves the clock turn, the gate and the `--long`
lane of ADRs 1684 and 1756 as they are. Context: ADRs 1646, 1659, 1684, 1706, 1756, 1766; traps 13,
119, 130.
Code: `tools/bounded.sh` (`queue_ask`, `queue_look`, `queue_leave`, `proc_start`, `lock_wait`,
`queue_patience`, `HEAVY_WALK_POLL`, the walk branch of `lock_take`, `lock_record`, self-test cases 13
and 14), `tools/state.sh` (`lock_cost`'s stopped runs), `doc/environment.md`'s rule line, `doc/todo/02`
section 0 item 3.
Tests: `tools/conformance/tests/bounded.rs` (the self-test, and
`the_lock_cost_lists_each_wait_that_ended_before_a_grant`). Instrument: ADR 1706's replay as kept at
`/home/AI/lock-replay-1472/` (`table74.py`), run unchanged.

## 1. The two holes

- **A later ask could take a lane first.** A walk polled the gate and its lanes twice a second and
  took whichever was free when it looked, so a lane freed while earlier walks slept went to whoever
  looked next. On batch seventy-four's log, round 1470's small walk asked at 23:01:16 queued
  **4 339.7 s**; round 1468's `raster_golden`, asked at 23:12:20, queued 490.8 s and its `corpus`,
  asked at 23:21:38, 3 056.4 s, both `behind=` the same census, campaign and export.
- **A wait that ended before a grant was on no line.** `lock_record` wrote at a hold's end and
  returned where nothing was held. Round 1469 stopped a `--tree 6` request after about ten minutes
  and has two lines, both later; round 1472's `tools/batch.sh check` died at its 1 500 s `timeout`
  queued for lane 1 and is on no line. And a clock run queued in a foreground `flock` ran no trap until
  its grant: stopped with `TERM`, it went on waiting, then exited 143 on no line.

## 2. The rule

- **Every walk — small, large, long — leaves a ticket** in `<lock>.queue` when it asks: its wrapper's
  pid, that process's start time (the 22nd field of `/proc/<pid>/stat`), the lanes it takes and its
  holder word. **A walk is granted lane N only where no live ticket asked before its own takes N.** A
  small walk behind a large one's ask therefore keeps the second lane, which the large one does not
  take; a large walk behind a small one's ask waits for it on the first.
- The file is read and written under its own `flock`, for one look and at most one grant, and never
  across a wait. A ticket whose pid is gone, a zombie, or names a process started at another time is
  dropped by the next look; a stopped process's (`T`) holds back nobody while it is stopped. A look
  that cannot have the queue's lock in `queue_patience` (2 s) is given up, said, and retried: no grant
  without it, and the waiting wrapper stays interruptible.
- **Clock runs take no ticket.** The clock turn and the gate order them as before, and a waiting clock
  run still stops every grant.
- **A wait stopped before its grant writes its line**: `hold=0.0s`, the wrapper's own status, `lane=-`.
  Every blocking `flock` of a clock run's path runs in the background under `wait`, so a trapped
  signal ends the wait at once; the exit trap ends the waiter, which holds a copy of every lane already
  granted. `SIGKILL` writes nothing and leaves its ticket for the next look.
- `tools/state.sh gates-cost` marks such a line `[stopped before a grant]`, lists the batch's, and
  sums their queue once more on its own; each still counts in its round's queue.

## 3. What it gives on batch seventy-four's lines

`table74.py batch-1468-1473` replays the 19 lines of the rounds, the arms export and the check, each
round's walks at their observed asks and holds. Its "observed order" replay — grants in the order the
log says the pollers won — is the race; its default is ask order, which is this rule.

| rule | 15 walks' queue | median | max | over 1 800 s |
|---|---|---|---|---|
| observed on the log | 10 935.0 s | 0.0 s | 4 339.7 s | 2 |
| replay, the pollers' race (calibration: 15 of 19 lines within 1 s) | 11 125.2 s | 45.3 s | 4 293.0 s | 2 |
| **replay, in the order asked** | **10 202.5 s** | 210.8 s | **1 932.7 s** | 2 |

The total falls 922.7 s and the longest wait by more than half; the median rises, because the order
spreads a queue that the race gave to a few. That is the trade: no walk waits behind those asked after
it, and the queue the batch pays is shared by when each asked.

## 4. Calibrated (trap 13)

Each assertion was planted out and seen to fail: the order look emptied — `order: walk 135 was granted
the second lane while walk 133, which asked before it, still waited`; the stopped line removed — `stop:
a walk stopped while it queued left no line`; the clock run's `flock` back in the foreground — `wrote its
line only once the holder had ended`; the dead-ticket drop removed and the stopped-process skip removed —
each `the walk asked after … was never granted the lane`. The case's walks look every 2 s
(`HEAVY_WALK_POLL`) and the last asks while the freed lane is free, the window the race lost in. The
order the queue gives does not depend on that timing, so the case cannot fail on a slow machine; only
its calibration's strength does. Three runs side by side passed.

## 5. What it costs

- A walk owed a lane takes it at its next look, so a grant can come up to a poll (0.5 s) later than the
  race would have given it, once per ask ahead of it.
- A wrapper stopped inside its look stops every walk's grant until it is continued or killed; the look
  is a few milliseconds a poll, and the waiters say so after 2 s.
- Until the merge, a wrapper from another checkout takes no ticket and races as before.
- A walk killed with `SIGKILL` is still on no line; nothing in the process can write one.

## 6. Re-ask

After the next batch, `tools/state.sh gates-cost` lists each stop. A walk granted (ask plus `wait=`) after
a walk of the same lane that asked later is a defect of this rule, not a price; and if the batch's walks
queue more than `table74.py`'s race replay of the same lines, the order is priced again against it.
