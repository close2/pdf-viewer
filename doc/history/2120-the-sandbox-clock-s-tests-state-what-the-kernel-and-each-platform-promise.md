# Round 2120 — the sandbox clock's tests state what the kernel and each platform promise

An instruments round in the `audit` worktree on CI run 44584637336 (main at d1cd577e), red in three jobs
on two `pdf-sandbox` tests (ADR 3181).

## Fixed

- **Linux, `clock::tests::a_thread_is_named_and_its_processor_time_is_read_from_another`**: it asserted
  a joined thread's `/proc/self/task/<tid>/stat` was gone at the join. The kernel wakes `pthread_join`
  when it clears the thread's id word in `exit_mm` and removes the task directory later, in
  `exit_notify`. A probe (spawn, join, read at once, 20 000 times) found the file still there 5 821
  times idle and 13 824 beside 40 spinning threads. The test binary run as `cargo test` runs it (the
  module's tests in parallel) failed 2 runs of 100 on the old test and 0 of 100 run alone, which is
  nextest's way; the test now waits for the reading to go after the join under a minute's ceiling, and
  failed 0 of 300 module runs and 0 of 50 whole-binary runs.
- **macOS and Windows, `confinement.rs`'s deadline test**: it asserted `TimedOut` everywhere. ADR 3133
  makes the wall clock the whole budget where `/proc` cannot be read, refused as `Stalled`; the test
  now states `TimedOut` on Linux and `Stalled { after: DEADLINE }` elsewhere. The code is unchanged.
  `cargo check --target x86_64-pc-windows-gnu -p pdf-sandbox --tests` with `-D warnings` exits 0.

## Ledger

None touched: the change is two tests and an ADR.

**Gates.** fmt 0 and clippy `-D warnings --all-targets` 0 over `pdf-sandbox`, `pdf-model`;
`pdf-sandbox --bins` 0; nextest `--lib --bins --tests` of the two, 2953 run, 2953 passed, no difference
from `expected-failures-current.json`; `cargo test -p pdf-sandbox` 0, 73 passed; rustdoc `-D warnings
--document-private-items` 0; the Windows check 0; `cargo test -p conformance` 0 once this paragraph was
written (`records` had failed for its absence).

## Trap

A thread's `join` is not the end of its `/proc` entry. A test that passes under nextest (one test per
process) and fails under `cargo test` (threads in parallel) is a race whose window the scheduler sizes;
reproduce it by running the libtest binary a hundred times, not once.
