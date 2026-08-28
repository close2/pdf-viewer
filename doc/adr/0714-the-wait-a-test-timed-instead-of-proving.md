# 0714 — The wait a test timed instead of proving

Status: accepted.
Context: `viewer-host drawing::tests::a_launch_waits_for_page_one_instead_of_polling_for_it`,
which guards ADR 0678's launch fix and had failed in at least five separate rounds' workspace
runs under sibling load (the history files of 765, 766, 771, 772 and 773 each record one;
772's carries a diff-reverted A/B proving the failure machine-shaped rather than tree-shaped).

## The problem

The requirement is real and is not weakened here: **a launch waits for page one instead of
polling for it.** `Drawing::settle(budget)` exists because a poll waiting on a toolkit's main
loop inside its own first frame cannot be dispatched (trap 21), and the wait repaired a
measured 44 ms GTK launch regression (ADR 0678; `doc/history/759`).

The test, though, asserted the *machine* along with the requirement: it passed
`Drawing::SETTLE` — one 60 Hz refresh, 16.7 ms of wall clock — as the budget and demanded a
real page's raster inside it. Alone, that holds by two orders of magnitude: the page draws in
about 3 ms. Beside a full `nextest --workspace` run and three sibling rounds' builds, the
drawing thread loses its core, the 16.7 ms runs dry, `settle` returns empty exactly as its
contract says it must, and the test reads a correct mechanism as a failure. Its own doc
comment predicted the mechanism. Five rounds recorded it; two explicitly left it "for a
viewer-host round."

749's rule names the defect: **a duration on a shared machine measures the machine.** Whether
16.7 ms suffices today is a fact about the scheduler's generosity, not about this code.

## The decision

The test now proves the waiting **structurally**, with no clock in any assertion. Three
choices carry it:

- **The test never polls.** No loop, no sleep: one `ask`, one `settle`, and the page must be
  in that one call's answer. Arrival inside the single call *is* the requirement — under the
  pre-settle shape the page can only come back through a later `collect`, which the test
  never makes.
- **The budget passed is the test's own give-up bound rather than `Drawing::SETTLE`.** What
  `SETTLE`'s value buys is a launch-latency choice, documented on the constant and accounted
  for by its two sibling tests (`the_budget_is_spent_once_over_a_whole_launch`,
  `a_page_that_outlasts_the_budget_is_not_taken_back`), both of which assert on what was
  *spent* and *answered*, never on elapsed time. Whether the constant's 16.7 ms is enough on a
  given day is precisely the machine fact the old shape kept measuring.
- **The page is expensive on purpose** — 5 000 page-covering fills, about 0.4 s of drawing on
  a quiet machine — so it provably cannot be finished in the microseconds between `ask`
  returning and `settle` blocking. That is what closes the one race a polling `settle` could
  have slipped through: a scheduler that stalled the test thread after `ask` long enough for
  the draw to land would hand a poll the page by luck. For that fluke the test thread would
  have to stay descheduled, mid-function, for the whole of a draw that itself needs half a
  second of CPU from a starved sibling thread — and the fourth assertion forecloses even
  that, because `spent` moving off zero is the wait itself having happened, which a poll
  never records however lucky its timing. The `spent` assertion is skipped where no drawing
  thread exists at all: a machine that refuses the spawn draws synchronously inside `ask`,
  a launch that waited for nothing because there was nothing to wait for.

## Calibration, per trap 13

A sweep for a defect must be run against the defect before it is believed, and a test is a
sweep with one hit. All four arms were run in one sitting, on one tree, with load supplied by
this round's own CPU burners (started and stopped by recorded PID):

| arm | quiet | loaded |
|---|---|---|
| old shape, unmodified tree | passes | **4 of 8 fail** at 48 nice-19 burners, load ~19 |
| new shape, planted pre-settle poll (`settle` degenerated to `collect`) | **fails 4 of 4** | **fails 10 of 10** at load ~33 |
| new shape, real tree | passes 3 of 3 | **passes 20 of 20** at nice-19 load to ~46, **10 of 10** at nice-0 load to ~54 |
| whole `drawing::` module, real tree | passes | passes twice at load ~54–57 |

The first row reproduces the five rounds' observation before anything changed; the second is
the planted defect failing deterministically under exactly the load that used to produce the
false failure; the third is the load-robustness the rounds asked for.

## What is honestly still wall clock

Two irreducible wall-clock elements remain, both bounded rather than hidden. The generous
budget and the test's `GIVE_UP` bound (two minutes) are *liveness* bounds — a hung thread must
eventually fail the test — and a liveness bound loosened by load can only turn a pass into a
slower pass, never a pass into a failure. And `spent > 0` reads a clock, but asserts only
that it moved during a block, a fact no load can falsify in the failing direction. What no
assertion states any more is that any particular duration was *enough*.

## Consequences

- The launch flake's standing entry in five history files is closed by change rather than by
  attrition; future workspace runs beside sibling rounds no longer carry a known false red.
- `Drawing::SETTLE`'s value is asserted nowhere and documented on the constant, which is
  where a product choice belongs. A regression in the *value* (say, someone setting it to
  zero) is caught by `the_budget_is_spent_once_over_a_whole_launch` only if it changes
  spending semantics; a deliberate retuning of the constant is a decision for a measuring
  round with the real hosts, not for a unit test on a shared machine.
- The pattern generalises and is 749's, restated for unit tests: **assert arrival and
  ordering inside one run; leave durations to gates that own a quiet machine.**
