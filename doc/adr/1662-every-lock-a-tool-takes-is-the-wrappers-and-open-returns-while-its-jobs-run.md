# 1662 — Every lock a tool takes is the wrapper's, and `open` returns while its jobs run

Session 1413. Status: **accepted** and **built**. Finishes what ADR 1646 section 1 left owed to
`tools/batch.sh` and `doc/environment.md`; corrects how ADR 1650 section 3.2's "`open` starts it
detached" was built. Supersedes nothing.
Code: `tools/batch.sh` (`detach`, `warm`, `arms_at_open`, `export_arms`, `holds_the_lock`,
`arms_held`, `run`, `gates`), `doc/environment.md`'s rule line, `tools/state.sh` (`lock_cost`'s note).
Tests: `tools/conformance/tests/batch.rs` (`open_returns_while_the_warm_build_and_the_arms_export_run_detached`,
`the_wrappers_last_line_names_the_seconds_gates_reads`), `tools/conformance/tests/bounded.rs`
(`every_lock_a_tool_takes_is_taken_by_the_wrapper_that_logs_it`).

## 1. `open` held its caller for the export's whole half-hour

`warm` and `arms_at_open` each started their job as `(cd "$wt" && setsid nohup <job> > log 2>&1 < /dev/null & echo …)`.
`&` binds looser than `&&`, so the `&` backgrounded the whole list: a copy of the shell ran `cd`, then
ran the job in its foreground and waited for it, holding the caller's standard output, which the
redirection on the job alone never closed. A caller reading that pipe to its end — a command
substitution, `Command::output`, an agent's shell — waited for the job. Batch sixty-four's `open`
was observed so: the copy of the shell, pid 2859031, reparented to `systemd --user`, in `do_wait`
after 756 s with descriptors 1 and 2 on the caller's pipe, until the export ended at 17:52, its lock
held 1 659 s. Reproduced alone: the old construction around a 6 s `sleep` returned in 6 012 ms, the
new one in 3 ms.

**Decision.** One `detach LOG WHAT COMMAND…` starts both jobs: `cd` and `;`, then
`setsid nohup COMMAND > LOG 2>&1 < /dev/null &` alone in the background, and the pid printed.
The warm build is started first and the export second, **beside it rather than after it**: the warm
build is the rounds' and runs at the caller's priority, the export's builds and walks at the
wrapper's nice 19 behind it, and the export's own build is the window in which a round's first edit
makes it refuse (`tree exit 1`) — this batch's warm build took 111 s and the export's build, beside
it, 102 s, so a serial start would have widened that window to about 210 s.

**Calibrated** (trap 13): with the old construction planted back into `detach`, the test's `open`
took 122.7 s against two stand-in builds of 60 s and failed; with `detach` it passed, the whole file
in 1.1 s.

## 2. The merge's gates and the arms export take the lock through `--lock`

Both took `/home/AI/heavy-walk.lock` with a bare `flock`, so neither hold was a line of
`/home/AI/heavy-walk.log`, and the export's half-hour — the longest hold of every batch — was the
one `tools/state.sh gates-cost` could not see.

- **`run`** runs each gate as `tools/bounded.sh --lock --round <the batch's branch> --nice 0 -- <gate>`.
  The wrapper's ceilings are the ones every round already runs these gates under. Niceness 0 because
  a timing gate's band is a claim about the program a person runs. The two clocks are read off the
  wrapper's last line, `bounded: … after <n>s`, which is the time from the lock's grant to the end:
  `wall` is that, `wait` the rest. A stamp written by an `sh -c` in front of the gate would have put
  fifty bytes of itself ahead of the gate's command in the log's `cmd=`, past the ninety
  `lock_cost` prints. The second test holds the wrapper's sentence and `run`'s reading of it together.
- **`arms`** checks the tree and the directory, then runs this script's `arms-held` under
  `tools/bounded.sh --lock --round arms --tree 12`; `arms-held` refuses where no ancestor holds a
  descriptor on the lock. One hold, one line, as before one hold and no line.
- **`doc/environment.md`'s rule line** spells the lock `tools/bounded.sh --lock --round <session>`.

`tests/bounded.rs` fails any script under `tools/` but the wrapper that runs `flock` itself, and a
rule line that spells the lock with `flock`. Calibrated: before this change it named
`tools/batch.sh:171` and `:241`, and its reader names a planted `flock` on a line of its own, behind
`exec` and inside a command substitution, and passes a comment.

## 3. Not changed

`fuzz/seeds.sh`'s header and `doc/verify.md` still tell a person to run a command under a bare `flock`;
they are instructions, not locks a tool takes, and they are their owners' to re-spell.
