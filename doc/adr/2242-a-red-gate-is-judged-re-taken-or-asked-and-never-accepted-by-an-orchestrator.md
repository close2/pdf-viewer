# 2242 — A red gate is judged, re-taken or asked, never accepted by an orchestrator, and batches 104–108 stand

Session 1704, slot 1 of batch one hundred and nine. Status: **accepted**. Prose and measurements:
`doc/reviews/1704-batches-104-to-108-against-the-principles.md`. Scripts: `doc/reviews/1704/`. Context:
principle 2 ("A regression fails the gate"), habit 101, ADRs 1023, 1748, 1863, 1969, 2209, 2221, 2233.
It cites no clause and moves no ledger row. ADR 2243 is unused.

## 1. The question

The owner asked on 2026-10-22 whether what another orchestrator did in batches 104–108
(`5ded4916..ee365e4f`) meets the project's standards. Review 1704 answers it finding by finding. This
ADR records the four decisions a later batch must not re-litigate.

## 2. Decisions

1. **Batches 104–108 stand. Nothing is reverted.** The code meets the standard. No `unwrap`, `todo!`
   or swallowed error was added to production code, every added `pub` item is documented, and the
   colour fixes were preceded by independent controls whose expected pixels come from the fixtures'
   encoded values. The arms held byte for byte from `020a86f3` to HEAD, and the five pages batch 104
   moved are each claimed. One exception goes to batch 110 as a slot, not as a revert: the owned
   `Conversion` built on every mark (review section 2, finding 2). The paperwork does not meet the
   contract (findings 3–5 and 7), and the remedy is forward. Records and ADRs are never rewritten.

2. **An orchestrator does not accept a red gate by ADR.** A gate red at the merge takes one of four
   answers, and the batch commit names which:
   - **the machine's**: HEAD in the same `--clock` hold is red by the same amount, and the
     instruction count beside a clock figure is inside its band (habit 101). The batch merges once on
     that sentence.
   - **persistent**: when the same gate is red the machine's way at a second merge running, its bands
     are re-taken by ADR 1969's rule, or its instrument is recalibrated, before the merge. A gate that
     is red at every merge detects nothing, and that is what batches 107 and 108 merged without.
   - **the batch's**: the cause is fixed, or the change is taken back.
   - **none of these shown**: a `Q` file to the owner, and the batch is held. A departure from a
     principle is the owner's to grant (CLAUDE.md: "say so explicitly rather than quietly
     compromising").

   ADRs 2209, 2221 and 2233 took none of the four. Their timing figures met the first; ADR 1969's
   rule was owed from the second merge. Their memory figures failed HEAD's comparison and needed the
   third or the fourth.

3. **An ADR states a decision under a Status line, and its title names that decision.** An assessment,
   a survey, a test plan or a diagnostic experiment is a review in `doc/reviews/` or a record's
   paragraph. An acceptance of a failed gate or a measured cost is named in the title of the ADR that
   makes it. Of ADRs 2178–2241, fourteen have no Status line, five are frozen at a state the merge
   passed, nine decide nothing, and the three red-gate acceptances sit under titles about stability.

4. **What a claim rests on is in the tree, or is reproduced by a command in the tree.** This covers a
   record's, an ADR's or a batch commit's claim. A path under `/home/AI/` or `scratchpad/` is a
   pointer for the session that wrote it, never the evidence. The owner cannot read the one, and the
   other is gone by the next batch. A batch commit states every round's duration and tool-use count,
   from the harness notification, as `doc/orchestrator-handover.md` section 3 step 4 says.

## 3. What follows, and whose it is

- Review 1704 section 4 orders batch 110's slots. The launch gate's cause is slot 2 of batch 109
  (session 1705).
- The lines that carry decisions 2–4 into `doc/todo/02` section 8 and `doc/todo/_brief-template.md`
  are the orchestrator's to place. This slot owns only the review, this ADR and its record.
- The enforceable halves are a conformance check that a new ADR has a Status line and that no new
  record or ADR names a `scratchpad/` path. Each is a batch 110 slot.

## 4. What it costs

- Decision 2 can hold a batch at the merge. That is the cost principle 2 already chose, and ADR 1969
  shows the re-take costs one quiet hold.
- Decision 4 moves receipts into the tree, as reviews 1456 and 1513 already keep their scripts. A
  receipt too large to commit is replaced by the command that reproduces it.
