# 1647 — Three bands the clock crossed, read again on a quiet machine

Session 1405. Status: **accepted**. Answers the three excursions round 1398 saw on HEAD:
`issue14415.pdf`'s step at 10.73 ms against 6.90 .. 10.40, `bring_up_ms` at 20.70 against
13.6 .. 20.6, and `bug1815476.pdf`'s `peak_anon_mib` at 46.6 below 47.5. Builds on ADRs 1556,
1577, 1585, 1621; trap 110. Supersedes nothing.
Code: none. `doc/checks/launch-path.toml` (the `bring_up_ms` paragraph, `bug1815476.pdf`'s
`peak_anon_mib` and its paragraph); `doc/checks/turn-path.toml`'s row is slot 3's file, and its
sentence goes to it through the orchestrator.

Every figure below is HEAD's (`0f9c12cf`), from a private worktree built `--release` into a target
directory of its own, behind the heavy-walk lock, at a one-minute load of 0.4 to 3.3 unless said.

## 1. The step: not reproduced, the band keeps its reason

Ten `turn_path` runs read the step at 8.37 to 8.66 ms, 33 of 33 figures judged in every run, 0
outside; fifteen sets of the gate's own three children, ten of them each born after 1.5 s of idle,
read 8.31 to 8.69. The band's 8.22 .. 8.62 was the same quantity. 10.73 did not recur on a quiet
machine with or without the idle before it, so it was the busy end of trap 110, and the gate's own
warm-up (ADR 1577) already holds the idle end.

## 2. Bring-up: the device's idle state, not the processor's clock; the band keeps its reason

Sixteen `launch_path` runs with clocks: 15.51 to 19.76 ms in fifteen, 21.26 in one, judged
and outside. The gate's own children, one at a time, nine to a set:

| arm | sets | minimum of each set |
|---|---|---|
| HEAD, born after 1.5 s idle | 10 | 19.93 to 25.45 |
| a 30 ms spin on the pinned cores first, after 1.5 s idle | 10 | 19.41 to 24.45 |
| HEAD, back to back | 5 | 16.00 to 17.72 |
| the spin, back to back | 5 | 16.68 to 17.32 |

Averaged over 24 children an arm, idle-born children read 4.4 ms more than back-to-back ones, the
adapter check 2.5 of it, instance creation and `request_device` the rest. A spin moves nothing, so
the 30 ms warm-up the warm open and the turn gate use cannot hold this figure, and it was not
built. The calibration probe reads 0.91 ms in seven of the ten idle sets and declines them; the
three it admits in each arm read 19.4 to 21.9, which is the excursion the gate can judge.

The file's clock rule over the sixteen runs would put the ceiling at 25.5 ms, inside the 25.2 to
28.3 an instance that loaded GL read (ADR 1532) — the regression this band exists to name as
itself. One run in sixteen past the edge by 0.66 ms is not a band failing on alternate runs, so it
stays 13.6 .. 20.6 with this reason written beside it.

## 3. `bug1815476.pdf`'s allocations: two modes, and the floor moves by the file's rule

Whether page one's stencils (decoded on the interpreter's thread) and its other images (decoded
ahead of their `Do` on the pool, ADR 1321) hold their transient buffers at once is decided by which
thread gets where first. The figure has two modes: the sixteen gate runs read 52.81 to 55.60 MiB in
fifteen and 45.91 in one; forty idle-born children read 53.5 to 54.7 in thirty-nine and 46.3 in the
first of them; at load 14, fifty-four children read 51.9 to 56.1 whatever the pool's width. The low
mode is the figure from before the two ran side by side, 44.5 to 46.2 (the row's own paragraph).

So the floor is this file's allocation rule over the new smallest: 45.91 less a tenth, rounded
down to half a mebibyte, **41.0**. The ceiling stays at 59.0, 3.4 MiB above the largest reading of
either mode. The counted run (no clocks, one child a figure) judges whichever mode its one child
draws, which is why the floor has to hold both.

## 4. What is not answered

Which kernel or firmware state the adapter check pays for after an idle second — the GPU's power
gating is the obvious candidate, and `strace -T` of the ioctls an idle-born child makes beside a
back-to-back one would say — and whether a launcher could keep the device awake as cheaply as a
spin keeps a core. Neither is this gate's to answer: a person's launch meets that state, and the
figure is right to contain it.
