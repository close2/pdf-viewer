# 0917 — The cold open gets a figure with no clock in it

Session 938. Status: **accepted**. The second of this round's two records;
[ADR 0916](0916-a-launch-clock-is-three-quantities-and-the-kernel-counts-one-of-them.md) is what a
launch clock turned out to be made of, and this is what the gate says instead.

## Context

`CLAUDE.md` principle 2 names **cold open** as one of four figures a perf gate owes, and
[ADR 0884](0884-the-launch-path-is-a-gate-and-a-wall-clock-band-needs-a-machine-under-it.md) made
it a wall clock with a band. Five rounds have now met that band and none has been able to trust it
on the machine it runs on; ADR 0916 says why, in three parts, of which only the first is this
program's.

The question principle 2 is actually asking a cold-open gate is **did opening a document become
more expensive**. A stopwatch is one way to ask it and it is the way that needs a machine nobody
has. This project has answered the same question with a count before — ADR 0180 localised 40% of an
open under callgrind, ADR 0667 took §7.4.4.4's predictor and ADR 0677 §7.5.8.2's `/W`, all three in
instructions — and `crates/pdf-syntax/examples/callgrind_open.rs` says why in its own first
paragraph: instructions are "unaffected by CPU frequency, background load or thermal state".

Nothing in the gate counted anything.

## Decision

**Two counted figures per document, judged on every machine under every load, beside the three the
gate already had.**

- **`open_kinstructions`** — thousands of instructions executed by a process that opens the
  document and stops, counted under callgrind. The child is a phase of the gate's own harness
  (`count-open`), so what is counted is the same `open_document` every clock figure times: no
  calibration probe, no `/proc` reading, nothing beside the open, because every instruction beside
  it is an instruction in the figure.
- **`read_calls`** — `/proc/self/io`'s `syscr` across the open. A cold open's elapsed time is the
  work plus one round trip to the disk for each read that misses the page cache; the *duration* of
  a trip is the machine's, and **how many trips** is the program's.

| | `open_kinstructions` | five consecutive runs | `read_calls`, five runs |
|---|---|---|---|
| `bug1815476.pdf`, 1 page | 3424.5 | 3 424 456 .. 3 424 586 (0.004%) | 25, every time |
| `PDF20_AN001-BPC.pdf`, 5 pages | 5046.8 | 5 046 740 .. 5 046 859 (0.002%) | 31, every time |
| `Well-Tagged-PDF-WTPDF`, 57 pages | 26 495.0 | 26 494 950 .. 26 494 986 (0.0001%) | 89, every time |
| `ISO_32000-2`, 1023 pages | 183 587.2 | 183 587 162 .. 183 587 294 (0.00007%) | 1077, every time |

The bands are the observed range widened by one per cent on each side — ninety times the measured
spread, so that this program's own hash seeds cannot fire it, and **forty times tighter than the
clock bands beside them**. About 976 thousand instructions of each figure are the process's own
start, which is 26% of the smallest row and 0.5% of the largest.

Three notes on making the count a property of the program rather than of the afternoon:

- **The child's environment is cleared.** Every byte of an environment is copied and walked at
  start-up, and the same binary counted under `cargo test` and counted from a shell differed by
  22 thousand instructions. A figure that moved with *how the gate was invoked* would not be what
  this row claims it is. `env_clear()`, then the two variables the phase needs.
- **The count is a number whatever the child did**, so the child reports its page count and the
  parent checks it: a process that failed to find the document executes fewer instructions and
  would read as a win (trap 16).
- **The profile is removed before it is written**, so that a run in which valgrind produced nothing
  cannot read the previous run's total and report it as today's (trap 10a).

Where valgrind is absent the run says so in a line of its own and the summary counts it. It is not
a failure — a machine without valgrind is a machine, not a regression — but it is not silence
either, and `doc/todo/02`'s own rule is that a gate which skips quietly is worse than one that was
never written.

## What this buys, and what the gate no longer leans on

**The every-round line now says something about this program on any machine.** With
`PDFVIEWER_LAUNCH_CLOCKS` unset the gate judges twenty-one figures — bytes, read calls,
instructions, what an open costs in memory, what page one allocates, what the device allocates — in
**three seconds at a load average of 6**, and it was green there. The nine-sample clock run under
the same load printed `NOT JUDGED`, named the probe, printed all thirty-seven figures and exited 0.
Both are the honest answer to their own question.

**It also gives the next round the instrument this one wanted and did not have.** `bug1815476.pdf`'s
cold open has sat a per cent or two over its ceiling since session 933, on quiet machines, with
every other probe in band; the two candidate explanations are that the program's open grew and that
the machine's small-read latency grew. This round could measure neither backwards. From now on the
first is one command and exact — and the evidence for the second is in ADR 0916: the same
measurement session 931 recorded at 0.109 ms reads 0.125 to 0.199 today.

**What it does not replace.** An instruction count says nothing about the disk, and a cold open is
partly the disk; it says nothing about how fast this machine executes, which is what a user feels.
The clock figures are still measured, still printed on every run, and still banded — they are asked
for rather than assumed, which is `doc/questions/A29`'s option 2 and not a demotion. What has
changed is that when the machine cannot support a claim, the gate still makes one.
