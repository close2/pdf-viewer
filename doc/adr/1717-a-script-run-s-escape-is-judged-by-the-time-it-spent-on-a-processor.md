# 1717 — A script run's escape is judged by the time it spent on a processor

Session 1442. Status: **accepted** and **built**. Context: ADRs 1590, 1609, 1694, 1716;
`fuzz/fuzz_targets/script.rs`; `doc/verify.md`'s `script` line.

## The finding

After this round's campaigns, `fuzz/seeds.sh check script` came back **not judged**: both its
`-runs=0` passes, over the disk corpus and over fresh seeds, stopped on the target's first property
— *a run took 2.664346841s* and *2.91993014s, past every budget the engine holds a run to*. The two
inputs are `seed_script.py`'s seeds at the element budget, `var a = new Array(1048576).fill(0);`
at the `WillSave` and `Format` sites. Four minutes later the same binary over the same disk corpus
at a load of 4.6 read `INITED cov: 13510` with no stop, and each of the two inputs runs in 0.42 s:
`-runs=5` on one of them takes 2 104 ms. So the run's own cost is 0.42 s, and the other 2.2 s or
more was the process waiting for a processor — the census runs at `tools/bounded.sh`'s nice 19
while siblings build on all 24 cores.

The 0.42 s is ADR 1590's and ADR 1609's known class, not a new one: one native call at a per-call
budget (`fill` over 2^20 elements) runs to completion because no slice yields inside it, past the
engine's 100 ms wall budget; ADR 1609 measured `join` over 2^20 at 160 ms and a typed array's
`join` at 372 ms, and the worker's 250 ms deadline is what bounds such a call, by a named loss. In
this instrumented build it is still under a quarter of the target's 2 s (`ESCAPED`, eight times
that deadline). Nothing in the engine changed; the census's two stops were the target's clock.

## Decision

`ESCAPED` is compared with **the run thread's time on a processor**, read from
`/proc/thread-self/schedstat`'s first field (nanoseconds) before and after the run, and the
property's message prints that time, the wall time, and the run queue's wait (the second field)
beside it. A run that escapes a budget spends processor time doing so; one that merely waited for a
processor enforced every budget it had. A run that *blocks* is not what this property is for —
nothing in-process blocks, and libFuzzer's `-timeout=20`, a wall clock, would name one. Where the
kernel offers no `schedstat` the wall clock judges, and the message says which clock did.

Why not raise `ESCAPED`: a larger wall bound still fails at some load, and a higher one hides an
escape on a quiet machine. Why not run the census at normal priority: nice 19 is the shared
machine's agreement, not the target's to override.

## Calibration (trap 13)

`ESCAPED` planted at 100 ms and the target rebuilt: the `fill(0)` input stops with *a run spent
428.129889ms on a processor (429.892666ms of wall time, 37.649µs of it waiting for one)*, and a
one-statement input passes; the plant reversed. So the property still fires on processor time, and
at an idle machine the two clocks agree to 2 ms.
