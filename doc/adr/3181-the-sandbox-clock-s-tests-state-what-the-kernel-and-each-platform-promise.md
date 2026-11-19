# 3181 — The sandbox clock's tests state what the kernel and each platform promise

Session 2120. Status: **accepted.** Context: ADR 3133 (the worker's processor budget, the wall clock as
a ceiling, `Stalled` where no reading is possible), ADR 3150 (an in-process decode read from
`/proc/self/task/<tid>/stat`), ADR 3130 (CI runs `cargo test`), `crates/pdf-sandbox/src/clock.rs`,
`crates/pdf-sandbox/tests/confinement.rs`.

## 1. Two failures on CI, neither in the code

CI run 44584637336 (main at d1cd577e) failed three jobs on two tests.

- **Linux**: `clock::tests::a_thread_is_named_and_its_processor_time_is_read_from_another` asserted that
  once `join` returned, the thread's `/proc/self/task/<tid>/stat` was gone; the runner read `Some(10ms)`.
  The premise is false. `join` is `pthread_join`, which waits on the thread's id word; the kernel clears
  that word and wakes the joiner in `exit_mm`, early in `do_exit`, and unhashes the task — removing its
  `/proc` directory — only in `exit_notify`, later. A probe (`spawn`, `join`, read the stat file at once,
  20 000 times) found the file still there 5 821 times idle, 5 305 times beside 8 spinning threads and
  13 824 times beside 40, its state `R` in nearly all of them and `X` or `Z` in the rest. Whether a run
  sees it is the scheduler's choice, which is why one machine passed and another did not.
- **macOS and Windows**: `confinement.rs`'s
  `a_decode_past_the_deadline_is_killed_and_the_next_request_answered` asserted `TimedOut` on every
  platform and got `Stalled { after: 250ms }`. ADR 3133 already decided this: where the worker's
  processor time cannot be read, the wall clock is the whole budget and its refusal is `Stalled`,
  "because that is what it is". The code answered as decided; the test stated Linux's answer for all.

## 2. The decision

- The clock test states the order the kernel promises: the reading goes **at some point after** the
  join, waited for under a minute's wall-clock ceiling that only a directory that never goes reaches.
  Nothing in `clock.rs` or `in_process.rs` relies on a reading vanishing at a join: `next_wait` treats a
  missing reading as "wait on to the ceiling, and let the pipe or the channel say how it ended".
- The deadline test states each platform's answer: `TimedOut` on Linux, where `/proc` is read, and
  `Stalled { after: DEADLINE }` elsewhere, the wall clock's refusal after exactly the budget. The code is
  not made to answer `TimedOut` everywhere: a refusal by the wall clock counted as the image's would let
  `raster_golden` and the oracle compare a page the machine decided, which is what ADR 3133 exists to
  prevent.

## 3. The cost

The clock test may spin a few more yields after the join. A Linux parent that cannot read `/proc` (a
`hidepid` mount) would answer `Stalled` and fail the deadline test; that is a machine this tree's tests
do not run on, and the failure would name the variant.
