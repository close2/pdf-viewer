# 1476 — A batch's clock is its gate log and its commit message, never a document

Session 1320. Status: accepted. Code: `tools/batch.sh` (`run` records each gate's `wall` and
`wait`; `gates` sums them on its last line), `tools/state.sh` (`gates-cost`). Prose:
`doc/todo/02` §8 item 4.

## 1. The question

Nothing in the tree says how long a round or a batch takes, or where a merge's time goes. The
orchestrator sees each round's duration and tool count in its notifications (this batch's briefs
cost one and a half to three hours and 100 to 350 tool uses a round), and `tools/batch.sh gates`
wrote one line a gate with an exit status and the test's own "finished in" — which leaves out the
build the gate line also pays, and says nothing of the time spent queued on the heavy-walk lock.
A round asked "which gate is dear" had no answer short of re-running the sequence.

## 2. Where the numbers may live

`CLAUDE.md`'s rule decides most of it: a fact that can be counted is not written down in an
instruction file; the command that counts it is. Durations are such facts. So:

- **No table of round or gate durations in any document**, `doc/history/` included as a shared
  file. A round's record stays the round's: it *may* say what one of its own runs cost where the
  cost is part of what it found, as any measurement, but it carries no standing clock line, because
  a round does not see its own start — the orchestrator does — and a figure the round guessed would
  be the one number in its record nobody measured.
- **The per-gate clock is the gate log's**, written by `tools/batch.sh gates` for every line:
  `wall` (the seconds the gate ran once it held the lock, build and walk together) and `wait` (the
  seconds it queued for the lock first). The two are apart because they answer different questions
  — `wall` is the gate's cost, `wait` is the other walks' — and a dear gate is read off the first.
  The last line states the sum of `wall`. The log is the orchestrator's (`/home/AI/batch-gates.log`
  unless `BATCH_GATES_LOG` says otherwise), overwritten each merge, and outside the tree.
- **`tools/state.sh gates-cost` prints it**, read-only, dearest first, with the log's own total
  line and the log's date. It does no arithmetic of its own (`tools/state.sh`'s header rule): the
  sum is `batch.sh`'s, and the order is a sort. A log written before the clocks existed is printed
  by the test's "finished in", and the section says that figure leaves the build out.
- **What survives a merge is the commit message**, a record no round reads to do its work: the
  log's last line, and one line per round with the duration and tool uses its notification
  reported, go into the batch commit's body. `git log --grep 'ALL GATES DONE'` is then the command
  that counts a batch's clock across batches, and the instruction files stay free of it.

## 3. What was not done

The orchestrator's per-round figures are not reconstructed for past batches: they were never in
the tree, and a reconstruction from memory is the guessed number section 2 refuses. The first
batch whose commit carries them is the first merge after this one.
