# 1646 — The lock a walk queues on is a number, and a launch child sets its own pool

Session 1405. Status: **accepted** and **built**. Builds what `doc/reviews/1401-are-we-making-progress-and-what-a-round-reads.md`
section 3 left owed (a `--lock` that logs the wait per command); takes `turn_path`'s rule about
`RAYON_NUM_THREADS` (trap 122) into `launch_path`. Supersedes nothing.
Context: the review's "two heavy-walk locks" row, declined for want of a count; ADRs 0798, 1476, 1612.
Code: `tools/bounded.sh` (`--lock`, `--round`, `lock_take`, `lock_record`, self-test case 7),
`tools/state.sh` (`gates-cost`'s `lock_cost`), `crates/viewer-ui/tests/launch_path.rs` (`run_phase`).
Tests: `tools/conformance/tests/bounded.rs` runs case 7 with the six before it.

## 1. The lock, logged by the program that takes it

`tools/bounded.sh --lock [--round N] … -- <command>` takes `/home/AI/heavy-walk.lock` itself, holds
it until the wrapper ends, and appends one line to `/home/AI/heavy-walk.log` when it does:

    <asked> batch=<branch> round=<N> wait=<s> hold=<s> exit=<status> cmd=<first 200 bytes>

- **The descriptor is inherited by the command**, as `flock <lock> <command>` leaves it, so the
  lock is held while anything the walk started still runs, a wrapper killed under it included.
- **An ancestor that already holds the lock is found and its descriptor used.** `flock` locks an
  open file description; a second description of the same file would queue behind the caller's own
  lock for ever, so `flock <lock> tools/bounded.sh --lock …` — the old spelling with the new flag —
  would hang. The wrapper looks in `/proc/$$/fd` for the lock file and locks that, which is granted
  at once.
- **The batch is the branch of the tree the script is in**, not of the caller's directory: a round
  measuring in a private export of HEAD calls the batch's wrapper from there (the first line this
  round logged said `batch=HEAD`).
- **The line is one write under `PIPE_BUF` with `O_APPEND`, made while the lock is held**, so two
  wrappers cannot interleave it. A signal ends the wrapper through `exit`, so the line is written.

`tools/state.sh gates-cost` prints the last batch's lines and each round's runs, queue and hold
summed; the sums are taken from the lines printed above them in the same run.

**Calibrated** (trap 13): with the lock never taken the case's "queued" line is missing and it
fails; with the inherited-descriptor search removed the third run times out at 20 s and it fails.

**What it measured on its first afternoon**: this round's second run queued 1 762 s behind the arms
export and a sibling's arms, to hold 82 s. That is the review's 350 to 3 900 s, now a line rather
than a sentence. Whether a second lock is wanted is the review's question, re-asked on these lines.

**Not changed, and owed to their owners**: `doc/environment.md`'s rule line still spells the lock as
`flock /home/AI/heavy-walk.lock tools/bounded.sh …`, and `tools/batch.sh run` takes it with `flock`
and logs its own `wait=` in `batch-gates.log`. A run that takes the lock with a bare `flock` is on no
line of the new log. Moving the rule line to `--lock --round <session>` is one edit.

## 2. A launch child's pool is the child's

`turn_path`'s children are given `RAYON_NUM_THREADS` equal to the cores they are pinned to (trap 122:
the lock's prefix of four reached a clock child and doubled the CPU backend). `launch_path`'s were
not: every child inherited whatever the caller had — four under the lock's prefix, one a CPU of the
machine under `tools/bounded.sh`, nothing in a bare shell — and the first page's interpretation, the
image decodes ahead of their `Do` and §8.7's shadings use the global pool. Now a pinned child gets
the pinned count and an unpinned one has the variable removed.

Measured on `bug1815476.pdf`'s first page, nine children an arm, HEAD's binary, load 13.7 to 14.9:
the allocated high-water read 51.9 to 55.3 MiB at four, 53.5 to 54.6 at eight and 54.2 to 56.1 at
twenty-four, the time to first page 28.8 to 52.8 ms across all three. So on this row the pool's
width is inside the figure's own spread, and the change is the instrument's correctness rather
than a finding about the row: the same binary no longer measures a different program depending on
who started it.
