# 1982 — "Granted at once" is the lock's own account, and a hold of nothing is nought

Session 1573, slot 3 of batch ninety-one. Status: **accepted, built**. Context: three merge chains in
a row met `tools/bounded.sh --self-test` failing with "lock: run 2 found the lock free or its caller's
and still waited", and each passed alone. Code: `tools/bounded.sh` (the self-test's `granted_at_once`
and its five call sites, three hang guards, and `lock_record`). It cites no clause and moves no row.

## 1. What the race was

The brief's hypothesis — a log line written after the holder's exit and read early — **did not
hold**: run 2 starts only after `wait "$holder"`, so nothing is reordered. The self-test judged "it
waited for nothing" by `wait=` under one poll, 0.5 s. But `wait=` is the wall clock from the ask to
the grant, and a run granted on its first look spends that interval on its own work: some twenty
processes (`flock`, `stat`, `awk`, `sed`, `tr`) and a `python3` for the suspend reading. That is the
processor's to give, and under load it gives less. Measured, a free lane's `wait=` read 0.0 s fifteen
times at a load of 26 on 24 processors; 0.5 s with six busy loops on the two processors the test was
pinned to, the self-test at nice 19 as every command under the wrapper runs; and 5.3 to 13.8 s with
the loops at nice 0, every line saying `behind=-`. The old script passed 2 of 2 runs beside a six-job
`cargo build` alone (a load of 5), failed 2 of 2 beside the loops, and 5 of 5 beside both — at the
lanes case (92), the order case (137) and a third site the brief had not named (§2). Not a lock
defect: every one of those runs was granted on its first look.

## 2. The decision

**The self-test reads "granted at once" off the lock's own record, `behind=-`.** A look that finds
its lane held calls `lock_queued`, which names the holder in `behind=` before the run sleeps a poll
or waits on the queue's lock; so `-` is exactly a run whose first look was granted, and it does not
depend on how long the look took. The five sites that compared `wait=` with a poll — run 2 and run 3
of the lock case, the lanes case's 92, the long case's 113, the turn case's 123, the order case's 137
— take it. Calibrated (trap 13): a copy whose first look misreads a free lane as held fails at run 2
with `behind=unknown`, and planted at round 92 alone, at the lanes case.

**A run that held nothing logs `hold=0.0s` because it held nothing.** The third failure was the stop
case's 142, a walk stopped while it queued, logged `hold=0.1s`: `lock_record` read the clock, then
`slept_ms`'s `python3`, then the clock again. The header states such a line says `hold=0.0s`, so the
end is the grant's reading where no lane was granted. This moves one figure the log carries, from a
tenth of a second under load to nought, and only on `lane=-` lines.

**The three hang guards are the minute the self-test's own comment states.** A run under its
caller's `flock`, a nested `--lock` and a long run under a bare `flock` were each bounded at 20 s; a
free run took 14 s at the harshest load measured, and a hang is for ever, so 60 s loses nothing.

## 3. Proof

The self-test twenty times at nice 19 on two pinned processors beside six busy loops and a six-job
`cargo build` of `pdf-syntax`, all pinned with `taskset` and each five runs one small walk of the lock:
the record carries the count. The old script under the same load, five runs: five failures.
