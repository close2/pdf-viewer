# 1794 — A script-worker test holds the question's clock, and waits on the worker's own death

Session 1479. Status: **accepted** and **built**. The shape is ADR 1780's, applied to the script
worker. Code: `crates/pdf-script-worker/src/client.rs` (`HeldClock`, `ScriptWorker::with_clock`,
`expire`), `crates/pdf-script-worker/tests/end_to_end.rs` (`OUT_OF_THE_WAY` and the four tests below).

## What was wrong

Two tests needed a question's wait to run out. They set it to 200 ms with `with_answer_wait` and
slept 300 ms past it, so each spent 310 ms (mean of 80 runs) sleeping, and their report assertion
named the 200 ms rather than `ANSWER_WAIT`.

Two tests had a verdict that was the worker's own death, but ran under the runner's 250 ms
`DEADLINE`, so they raced the watchdog:

- `growth_past_the_ceiling_ends_the_worker_and_is_named` asserts the allocator's abort. On one core
  shared with eight busy loops, the growth was killed at the deadline in 2 of 20 runs.
- `the_engines_library_runs_inside_the_filter` asserts that no script is the seccomp filter's kill.
  It failed in 8 of 80 runs at a load average of five to six, every time on
  `Deadline(250ms)`. The culprit is the 20 000-element `sort` with a script comparator. That
  callback from inside one native call is the work ADR 1590 says the engine's budgets cannot see, and
  it took 138 to 152 ms in the runs where it lived.

## Decision

**A question's wait is measured on a clock a test can hold.** `ScriptWorker::with_clock` takes a
`HeldClock`, which stands still until its holder calls `advance`. The runner reads it for one thing,
how long a question has waited, and reads `Instant::now` when it has none. The two withdrawal tests
use the real `ANSWER_WAIT`. They stand the clock one nanosecond short of the wait and see nothing
withdrawn, then move it on and see the withdrawal. No wire change: withdrawal is the host's half alone.

**A question is withdrawn at the moment `question_deadline` names.** `expire` compares with `>=`, so
a host woken at the deadline finds the card withdrawn and does not have to wait for its next poll.
Before this, an answer that arrived exactly at the deadline was still handed to the question.

**A test whose verdict is the worker's own death puts the deadline out of the way.** Both tests above
run with `with_deadline(OUT_OF_THE_WAY)`, which is one minute. The run then ends at the worker's own
signal however slow the machine is, and a worker that does not die fails the test on `Cause::Deadline`
instead of hanging it.

**Calibrated by breaking it** (trap 13). If `expire` reads `Instant::now` in place of the held
clock, both withdrawal tests fail, on "withdrawn as the wait ends" and on the late answer handed
over. If the boundary goes back to `>`, both fail at the same two assertions.

## Consequences

- Held, the two withdrawal tests take 4 and 5 ms (mean of 80 runs), down from 310 ms. The growth
  test failed 0 of 20 runs on the contended core and the filter test 0 of 80, each with the same load
  that failed them unheld.
- Four timed bounds stay, each passing 20 of 20 runs on the contended core:
  - `a_run_past_its_deadline_is_killed_named_and_the_next_trigger_starts_another`:
    `spent < 4 × DEADLINE`, where `spent` includes the spawn.
  - `a_question_holds_its_script_in_the_worker_and_the_answer_resumes_it`: the commit's
    `< 4 × DEADLINE`.
  - `a_run_the_engines_budgets_stop_answers_inside_the_deadline`: `slowest < DEADLINE`, which is
    ADR 1609's measurement and whose verdict is a time.
  - `after_its_losses_a_document_starts_no_further_worker`: its 50 ms deadline is a kill the test
    chooses on a script that cannot finish, so nothing races it.
- A form script that sorts 20 000 values with a comparator runs at 0.6 of `DEADLINE` on a lightly
  loaded machine. Whether such a run should be killed is a question about `DEADLINE` and ADR 1609,
  not about this test, and it is left there.
