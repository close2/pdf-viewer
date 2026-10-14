# 1903 — A lock line says what of it the machine slept, and a batch round says its own number

Session 1533, an instruments round of batch eighty-four. Status: **accepted** and **built**.
Context: ADR 1646 (the lock line), ADR 0963 (the session's two sources), trap 164. Code:
`tools/bounded.sh` (`slept_ms`, `slept_field`, `lock_take`, `lock_record`, self-test case 7),
`tools/state.sh` (`lock_cost`), `tools/round.sh` (the round's heading). Tests:
`tools/conformance/tests/bounded.rs`. Cites no clause.

## 1. The questions

Trap 164: a lock line's `wait=` and `hold=` are wall clocks, and a suspend is inside them. A build
cargo timed at 350 s logged `hold=10178s`, six arms 11 412 s, and a clock run queued 11 120 s behind
them, when the machine had slept about 9 800 s. Nothing on the line said so, so `gates-cost` summed
the sleep as queue and hold. `grep -n "slept\|BOOTTIME" tools/bounded.sh` printed nothing.

`tools/round.sh` computes the session as the last `doc/history/` number plus one, or takes a larger
number from a `round-<n>` branch. A batch branch, `batch-1528-1533`, holds six sessions that run at
once. Every round of the batch is told the first unwritten number, here 1528, whatever its brief
assigned. It once told a round the brief numbered 1516 that it was 1522.

## 2. Decision

1. **The wrapper reads `CLOCK_BOOTTIME − CLOCK_MONOTONIC` at the ask, the grant and the end.** Only
   a suspend moves that difference. Neither clock is in `date`, and `/proc/timer_list` is closed to
   this user, so python3 reads it (12 ms a reading, three a locked run). Where python3 is missing
   the reading is `-` and the line says nothing about sleep.
2. **Where the machine slept 0.1 s or more of either interval, the line says how long.** It writes
   `slept_queued=<s>` for the queue and `slept=<s>` for the hold, after `lane=` and before `cmd=`.
   The first six fields stay where `gates-cost` reads them by position, and `cmd=` stays last
   because it has blanks in it. A line with no suspend in it is unchanged.
3. **`gates-cost` takes each one off the figure it was inside.** The line is marked
   `[slept <s>, taken off]`, and the batch's total is printed once.
4. **`ROUND_SESSION` carries the brief's number to `tools/round.sh`.** The script prints it as the
   session, says what `doc/history/` alone would have said, and refuses a number outside a batch
   branch's range. With no variable on a batch branch, it prints the first-unwritten number and the
   branch's range, and says how to get the round's own number. A batch's prompt that says
   `ROUND_SESSION=<n> tools/round.sh <kind>` gets the right line. One that says only
   `tools/round.sh <kind>` gets both numbers, never only the wrong one.

## 3. Calibration

The self-test cannot suspend the machine, so `HEAVY_WALK_SLEPT_FILE` stands in for the reading. A
bare `flock` holds the case's own lock while a run queues. The reading moves 4 s before the holder
lets go, and the run's own command moves it 9 800 s more. The line reads
`lane=1 slept_queued=4.0s slept=9800.0s cmd=sh -c …`, and a run that moved nothing says neither. The
case's first draft let the holder write its reading after its own exit, so the run read none, and
the case failed and named the line. `the_lock_cost_takes_a_suspend_off_the_line_it_was_inside`
plants trap 164's two lines and sums 350 s held and 1 320 s queued. A third line with no sleep is
summed as written.

## 4. What it costs

Three python3 starts per locked run, about 36 ms against holds of seconds to hours. A suspend
under 0.1 s is not reported. The batch's prompt template is outside the tree. Until it says
`ROUND_SESSION=<n>`, a round reads two numbers and has to pick its own.
