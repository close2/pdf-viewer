# 1675 — A lock line that names no batch is counted by its round

Session 1419. Status: **accepted** and **built**. Answers the defect ADR 1659 section 1 reported to
`tools/state.sh`'s owner. Supersedes nothing.
Code: `tools/state.sh` (`lock_cost`). Test: `tools/conformance/tests/bounded.rs`
(`the_lock_cost_counts_a_line_that_names_no_batch_for_the_batch_its_round_is_in`).

## 1. The under-count

`tools/bounded.sh --lock` writes the branch of the tree it lives in as `batch=`. A wrapper run from a
detached checkout of HEAD writes `batch=HEAD`, and `/home/AI/heavy-walk.log`'s first two lines are
round 1405's, so written: 159.6 s and 1 762.3 s of queue. `lock_cost` took the last line's branch as
the batch and counted only lines naming it, so batch sixty-three's round 1405 read 6 runs and
2 586.5 s queued where ADR 1659 section 1 counts 8 and 4 508.4 s — **1 921.9 s** short. The brief
called the two lines the merge's own holds; their `round=1405` and their commands, under
`scratchpad/r1405/`, say they are the round's.

## 2. Decision

- **A line's batch is the branch it names when that is `batch-<first>-<last>`, and otherwise the
  batch on the log whose sessions hold its round.** Those are the same fact for every line a
  worktree's wrapper writes, and the only one a detached checkout's line carries. Such a line is
  printed with `[batch=HEAD, by its round]`, and the section ends with how many lines were placed so
  and their queue; a line whose round no batch on the log holds is counted for none and said once.
- **The last batch is the last branch of that shape on the log**, not the last line's, so a detached
  run appended last no longer makes the section print the batch `HEAD`.
- **The log is not rewritten**, and the wrapper still writes the branch it finds: the log is the
  wrapper's record and is read, never written (ADR 1646), and a detached tree has no batch to write.

**Calibrated** (trap 13): the test's planted log run through the previous `lock_cost` printed
`batch HEAD, by round:` and nothing of the planted batch. On the live log the section now prints
batch sixty-three's round 1405 as 8 runs, 4 508.4 s queued and 2 120.5 s held, ADR 1659's figures.
