# 3134 — The waiter's self-test orders events rather than counting seconds

Session 2108. Status: **accepted.** Context: ADR 2025 (a start returns at once, and the wait is the next command),
ADR 3114 (a detached job's mark is waited for until the job ends), `tools/wait.sh --self-test`,
`tools/conformance/tests/wait.rs`.

## 1. The failure

Round 2103's nextest, at a load near 200, failed `the_waiters_self_test_holds` on two cases: "a start and a wait in
one call took one wall" (`took 3 s`, against `-le 2`) and "a locked run behind a holder is started at once"
(`returned after 3 s`, against `[0-2] s`). Both asserted that `--start` does not wait for its run by counting
seconds, and a loaded scheduler starts a short `bash` late as readily as a wrong script would.

The proof is a stall injector: a `bash` earlier on `PATH` that sleeps four seconds before it runs a `--start` of the
case's `c.log` or `g.log`, as a loaded machine starts it late. The self-test before the change fails exactly 2103's
two cases (`took 5 s`, `returned after 5 s`); after it, every case holds.

## 2. The decision

What each case asserts is an order of two events, and that is what it now checks:

- **One call, one wall**: the run ends only when the test creates `c.go`, or at its own minute's ceiling. A call of
  `--start` and `--wait --wall 1` that returns 75 while the run has written no end line has waited neither for the
  run nor for the caller's pipe to close, however long the scheduler took to start either.
- **A locked run is started at once**: the holder lets go only when the test creates `go`, or at its minute's
  ceiling, so a `--start` that returned while the holder still held the lock and had not ended did not wait in the
  lock's queue.
- The two loops before them, which waited ten seconds for the holder to hold and for the queued line, wait for the
  event or for the run that would make it ending, with three hundred seconds as a ceiling only on a run that does
  neither — ADR 3114's form.

Calibrated (trap 13) with `--start`'s `&` removed, so that it waits for its run: the new self-test fails both cases by
their new sentences ("the run had ended when the call returned", "returned after the holder ended"), and the
queued case after them.

## 3. The cost

A broken `--start` now costs the self-test a minute per case, the runs' ceilings, where it cost three seconds. What
is still counted in seconds — `--start`'s five seconds for its run's pid file, the twenty-second walls of the cases
that wait for a short run to end — is the script's or a twentyfold margin, and is left.
