# 3150 — An in-process decode is budgeted in its thread's processor time

Session 2112. Status: **accepted.** Context: ADR 1447 (the in-process deadline abandons a thread),
ADR 3133 (the worker's processor budget and wall-clock ceiling), principle 3
(`#![forbid(unsafe_code)]`), `crates/pdf-sandbox/src/clock.rs`, `crates/pdf-sandbox/src/in_process.rs`.

## 1. The question

ADR 3133 left `decode_here_within` on wall clock, its `Overran` counted as the machine's. Can a
thread's processor time budget it the same way without `unsafe` in a crate that touches PDF bytes?

## 2. The answer: yes, through `/proc`

`clock_gettime(CLOCK_THREAD_CPUTIME_ID)` and `gettid` are system calls the standard library does not
wrap, so either needs `unsafe` or a crate that holds it. `/proc` needs neither: the decoding thread
reads the `/proc/thread-self` link (`<pid>/task/<tid>`) for its own id once, and its own
`/proc/thread-self/stat` when it takes a request; the caller reads `/proc/self/task/<tid>/stat` while
it waits. The fields are a process's (`utime`, `stime`), counted for the thread alone.

## 3. The decision

- The decoding thread sends `Taken { whose, at_start }` before it decodes, and the caller's wait is
  `clock::Allowance`, now shared with the worker: the thread's processor time on this request
  against the budget, behind a ceiling of `STALL_FACTOR` times it from the call's start.
- Spending the budget is `SandboxError::Exhausted { budget }`, **the image's refusal**, not counted;
  the ceiling is `Overran { after }`, the machine's, counted as before. Both abandon the thread, as a
  thread cannot be killed. Until the thread has said who it is, and wherever `/proc` cannot be read,
  the wall clock is the whole budget, refused as `Overran`, as before.
- `TooManyOverran` stays the machine's: whether an abandoned decode has ended by the next request is
  a matter of wall clock, whatever abandoned it.
- `clock::Ended` is what both paths share; `From<Ended>` gives the worker its `TimedOut` and `Stalled`.

## 4. The proof

A release build of one 8192² JBIG2 region (1.1 s of processor idle), budget 1.5 s, pinned to one
processor at `nice 3` beside a busy loop: before, `Overran { after: 1.5s }` at 1.51 s, counted;
after, decoded at 2.57 s and 2.65 s, nothing counted. Idle at a 0.5 s budget, after: `Exhausted`,
not counted. Its process stopped for 17 s mid-decode, after: `Overran { after: 15s }`, counted.

## 5. The cost

A starved in-process decode is waited for up to ten times its budget where it was once; a decode
that spends its budget is abandoned when it has spent it, on any machine. Off Linux nothing changes.
