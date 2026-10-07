# 1638 — A round is briefed from a template, reads a held figure, and runs on a clock

Session 1401. Status: accepted. Prose: `doc/reviews/1401-are-we-making-progress-and-what-a-round-reads.md`
(the measurements), `doc/todo/_brief-template.md`, `doc/todo/02` section 0 and section 8 step 2,
section 2 rule 4; `tools/round.sh`. Companion: ADR 1639 (what a round reads).

## 1. The question

The owner asked on 2026-10-07 whether the batches are making progress, how to raise what a round
produces and cut the work that produces nothing. The review measured batches fifty-three to
sixty-two and found the waste in five places, each with a number: a brief of 240 lines of which
149 were identical for sixteen batches and 155 retold lessons; four of six contracts in one batch
resting on a premise the tree contradicted; a batch whose six slots were busy 43–75% of the slot-
hours because nothing bounded its slowest round; one round in five told to run the merge's walks;
and five re-takes of a frame table a gate had banded since ADR 1513. This ADR decides what changes
in how a round is briefed and run. It changes no gate and weakens no rule of `doc/todo/02`
section 2.

## 2. Decisions

1. **The brief is written from `doc/todo/_brief-template.md` and from nothing else.** A common
   part of at most sixty lines that points at `doc/todo/02` section 0, `doc/environment.md`'s rule
   block and the two every-round files; a slot part of at most twenty-five lines carrying the
   contract, the owned files, **the premise with its evidence**, the reading beyond
   `tools/round.sh <kind>`, the gates owed, the wall budget and the assigned numbers. The
   orchestrator counts both with `wc -l`.
2. **A lesson is made once, in the tree, and the brief points at it.** What a batch teaches
   becomes a line in section 0, a line in the environment rule block, or a trap or habit proposed
   in a report — never a paragraph retold in the next sixteen briefs. Section 3 of the common part
   names where each landed, one line each, at most five.
3. **A premise is checked by the orchestrator before it is briefed**, in the text (`doc/md/`), the
   code (the refusal's call sites) and the data (a record's figure with its unit), and the
   evidence is written into the slot part. A round still checks it (`doc/habits/every-round.md`'s
   first line), and says in its report which held.
4. **A figure a gate already holds is read from the gate, never re-taken** — a band in
   `doc/checks/`, a ratchet's floor, a held list. A round re-measures what its own change can
   move, and moves a band only with the reason beside it (trap 97). Section 0 item 3 states it.
5. **Every slot carries a wall budget**: three hours by default, four for a build slot the
   orchestrator names as such. At the budget a round leaves every file it touched compiling and
   reports what is done; the next batch's contract takes the residue. The batch's clock is then
   its longest budget plus the merge, not its slowest round.
6. **The fifth-round rule does not bind a round run in a batch.** `doc/todo/02` section 2 rule 4
   now says so: the merge runs all three tiers on every batch, which checks the crate-graph map
   five times as often as the rule did, and a round's own tier 3 was the merge's work done twice.
   A round run alone, outside a batch, keeps the rule. `tools/round.sh` no longer prints it.
7. **The instruments slot's standing items rotate.** The four navigational documents are re-read
   every fourth batch rather than every batch; the frontier map is a gate and is not re-derived;
   otherwise the slot builds the instrument the last batch's records asked for.
8. **The merge is unchanged**: all three tiers, every batch, `tools/batch.sh check` before
   `commit`, the clock in the commit body (ADRs 1476, 1500).

## 3. Declined, with the reason

- **Two heavy-walk locks.** The gain is the lock wait, which no instrument records and three of 49
  records put between 350 and 3 900 s. The risk is the machine: 2026-09-02 reached 61 GB of 63 with
  one 32 GiB walk beside builds, and 2026-10-06's kill was a task count no memory bound saw
  (trap 116). Two 12 GiB walks beside six 8 GiB builds is 72 GiB nominal. First the count:
  `tools/bounded.sh` is to gain a `--lock` that records the wait per command under
  `scratchpad/r<round>/`, owed to the instruments slot; the question is re-asked when a batch's
  waits have been summed.
- **A change-reaches merge with a weekly full set.** It saves about twenty minutes of gate wall a
  batch and moves tier 3's catches — which `doc/todo/02` section 2 shows happen at merges — up to
  six batches later. Rule 5 stands.
- **Records as a table.** No round reads a record, so the shape costs the writer only, and the
  `records` gate reads `**Gates.**`. The budget stays; the record is written to the finding.

## 4. Owed, and to whom

- `tools/batch.sh arms`: the HEAD arms exported once per batch at `open` — digests per page per
  arm under the batch directory, keyed by `git rev-parse HEAD` — so that no pixel round exports
  HEAD again (five did in nine batches). The next instruments slot.
- `tools/bounded.sh --lock`, as above. The same slot.
