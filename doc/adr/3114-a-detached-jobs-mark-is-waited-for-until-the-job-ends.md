# 3114 — A detached job's mark is waited for until it is there or the job has ended

Session 2103. Status: **accepted.** Context: ADR 1662 (the export's hold is a line of the lock's log), ADR 1674 (the
wrapper keeps the lock), `tools/conformance/tests/batch.rs`'s
`open_returns_while_the_warm_build_and_the_arms_export_run_detached`, `tools/bounded.sh`.

## 1. The failure

The test failed twice at merge re-gates with "the export never wrote its lock line, so `arms-held` did not run under
the lock", and passed alone in under two seconds. It waited for each of the export's three marks — the sandbox's
heavy-walk lock taken, the README's `lock: queued` line, the lock log's `round=arms` line after the stop — for twenty
seconds counted as two hundred polls.

On this machine at 12:03 on 2026-11-19 the user slice stood at 65 GiB of its 66 GiB `MemoryHigh` (27 203 `high`
events), its processor pressure at `some avg10=89`, and the load average at 155 to 258. Three runs of the test
there: one failed with the merge's message at 22 s, two passed at 13 s and 5 s. A sampler of the failing run's
processes saw the export's wrapper and its subshell asleep for 20 s before `batch.sh arms-held` appeared, and
`arms-held` itself runs at the wrapper's `nice 19`. So the twenty seconds are a guess at the machine's load, and the
script does nothing wrong: the mark comes, late.

## 2. The decision

Each of the three waits is for an event: the mark, or the export's process group ending, after which the mark is read
once more, because the export writes its last mark before its last process exits (`marked_before_ending`). A loaded
machine delays the mark and the end alike, so neither outruns the other. An export that ends without its mark fails
the test at its end, as before; 300 s is a ceiling only on an export that neither marks nor ends.

What the test asserts is unchanged: the lock taken by the export, the README's lock line written under it, and the
hold's line in the log.

## 3. The proof

A stall injector (`SIGSTOP` of `batch.sh arms-held` before its README, `SIGCONT` 30 s later) stands in for the load:

| stalled | the test before | the test after |
|---|---|---|
| `arms-held`, after the lock | failed at 20.6 s, the merge's message | passed at 30.6 s |
| the wrapper, before the lock | failed at 21.4 s, "never took the heavy-walk lock" | passed at 30.7 s |

Calibrated (trap 13): with the README's `lock: queued` line misspelt in `tools/batch.sh`, the test after the change
failed with the merge's message at 124 s, when the export's two sixty-second stand-in builds ended.

The cost: a failing export now holds the test for as long as the export runs, a stand-in's two minutes, where it
held it for twenty seconds.
