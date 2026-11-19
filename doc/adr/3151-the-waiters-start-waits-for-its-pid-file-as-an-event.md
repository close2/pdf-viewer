# 3151 — The waiter's start waits for its pid file as an event

Session 2112. Status: **accepted.** Context: ADR 2025 (a start returns at once), ADR 3114 (a mark is
waited for until it is there or its job has ended), ADR 3134 (the self-test orders events),
`tools/wait.sh`.

## 1. The failure it would have

`--start` waited for its runner's `<log>.pid` by counting a hundred polls of 50 ms, and refused
"the run did not write … within 5 s" past them: a loaded machine that starts the runner late got a
refusal for a run that was in fact starting, and that run then ran on with nobody told to wait for it.

## 2. The decision

The wait is for the pid file, or for the runner ending, after which the file is read once more,
because the runner writes it before it can end. `setsid --wait` keeps the job this waits on alive
for the run's length where `setsid` must fork. A runner that ends without the file is refused when
it ends, with the log's last lines; 240 s, inside the call's 270, is a ceiling only on one that does
neither.

## 3. The proof

Two self-test cases, each with a `setsid` earlier on `PATH`: one holds the runner until `late.go`,
made six seconds after the shim is reached; one exits at once. The old `start` fails both ("did not
write … within 5 s" in each); the new passes both, the first returning only after `late.go` existed.

## 4. The cost

A runner that hangs before its pid file holds `--start` for four minutes where it held it for five
seconds; the self-test costs six seconds more.
