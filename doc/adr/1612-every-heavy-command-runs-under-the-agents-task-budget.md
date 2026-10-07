# 1612 — Every heavy command runs under the agent's task budget, and the owner's list names the agent's cgroup

Session 1388. Status: **accepted**. Carries out trap 116's rule as an instrument. Extends ADR 0798's
bound (`--data`, `--tree`) with a third, ADR 1601's owner's list with one item, and ADR 1313's
`check` with one line. Supersedes nothing.
Context: trap 116 (`doc/traps/instruments-and-reports.md`); the owner's note on the incident; ADRs
0798, 0807, 1440, 1601.
Code: `tools/bounded.sh` (`--tasks`, `--task-budget`, the task-limit report, self-test case 6),
`tools/batch.sh` (its head, `check_batch`), `tools/state.sh`, `tools/fuzz.sh` and
`tools/drive-windows.sh` (their heads), `tools/main-checkout.py` (`agent_scope`, `BOUND`).
Tests: `tools/conformance/tests/bounded.rs` (`every_heavy_script_in_tools_runs_under_the_task_budget`),
`batch.rs` (`check_prints_the_tasks_held_and_the_limit_where_called`), `owner_section.rs` (the
planted checkout's two scopes and a stranger's).

## 1. What no bound saw

On 2026-10-06 one tool forked a task per package and never waited. The agent's scope climbed about
3 400 tasks a minute to 52 259, with 50 GB resident and 91 GB of swap, and the system's OOM daemon
killed the whole scope. Each task was small. `RLIMIT_DATA` is per process, and the tree ceiling
samples resident memory with a `ps` that stalls under that pressure (ADR 0807). Neither is the
bound that acts on a process count.

## 2. The decision

**`RLIMIT_NPROC`, at 8192, for every heavy command.** `fork` and `clone` fail with `EAGAIN` once
the user holds that many tasks, at the call, with nothing to sample. The kernel checks the count of
every task of the *user*, so the figure is one budget for six rounds and the orchestrator together.
Held: under `--tasks 64` the self-test's fork loop is refused at its first fork, because the user
already holds more than 64 tasks. 8192 is far from a working batch: sampled every five seconds over
this round, while siblings built and tested, the user held between 120 and 647 tasks (439 samples). Trap 116's
re-run reached 7 845 under the bound before it was killed.

**Written once.** `tools/bounded.sh` holds `task_budget` and applies it with `prlimit --nproc`
beside `--data`. `--tasks` may lower it and may not raise it, because a higher limit for one command
lends it the other rounds' share. `--task-budget` prints the figure. `tools/batch.sh`,
`tools/state.sh`, `tools/fuzz.sh` and `tools/drive-windows.sh` read it there and set `ulimit -u`
before anything runs, keeping a lower limit the caller already holds. A refused fork in a failed run
ends on `STOPPED BY THE TASK LIMIT`. Only a failed run: a program that met `EAGAIN` on a non-blocking
descriptor prints the same words.

**`check` makes the bound legible.** It prints the user's tasks now and the `ulimit -u` of the shell
that called it, marked `ABOVE the budget` when it is. That is not a finding, since the script holds
itself to the budget anyway, but the gates the merge runs beside it are only bounded if the merge's
shell was.

**The cgroup is the owner's, and the list now says so.** The agent runs in a terminal tab's scope of
the owner's session, `…/app-org.kde.konsole-<pid>.scope/tab(<pid>).scope`. No rlimit of this tree
reaches the agent itself. `tools/main-checkout.py` reads the scopes from the processes whose real
uid is the agent's, because the tab changes every session. For each it finds the tightest
`pids.max` and `memory.max` on the path, since a limit on an ancestor bounds it too. A scope above
16384 tasks or 40 GiB is owed. 16384 is twice the rlimit, so the rlimit acts first and the cgroup
holds what no rlimit reaches. 40 GiB is ADR 0798's `MemoryMax`. The item writes both files of each
unbounded tab, or starts the next session in a scope of its own with `systemd-run --user --scope`
(ADR 0798's command, with `TasksMax`). No separate ADR 1613 was needed: the figures are this one's.

**The self-test no longer waits out its own abandoned samplers.** Case 5 redirected the call to
`watch_tree`, and bash keeps the caller's descriptors saved on 10 and 11 in every subshell it forks
for a sampler. The stalled samplers' `sleep 60` kept the output pipe open, so read through a pipe,
as `cargo test` reads it, the self-test took 71 s and printed its last line at 15 s. The case now
runs in a subshell whose descriptors are set with `exec`, and its verdict comes back in a file. It
takes 15 s, and `tests/bounded.rs` was the longest test in `cargo test -p conformance`.

## 3. The premises, checked

- *The limit is per user*: held, from the kernel's rule and from case 6 above.
- *Six rounds sit under 1 000 tasks*: held over this round, 647 at the most.
- *ADR 0798 says how*: held for a scope created at launch. For the running tab, ADR 0798's
  `systemctl --user set-property` names the konsole scope. The tab's own limit files are owned by
  the owner (`ls -l` on this machine), and its parent's `cgroup.subtree_control` enables `memory`
  and `pids`, so the owner can write them directly. Not attempted from this account.
- The konsole scope's own `pids.max` is 76003, systemd's default. That is above the incident's
  52 259, so it is reported as unbounded.
