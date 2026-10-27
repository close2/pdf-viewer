# 2464 — A narrowed brief opens on its frontier, and a row is briefed on what would move it

Session 1815, a review round beside batch one hundred and twenty-one. Status: **accepted** for rules
R1–R5, which the briefs carry from batch 122. Rule R1's last sentence stays **proposed** until the
owner answers Q681. Context: the owner, 2026-10-27, "investigating if we are making progress or if
we are again (slightly) stuck in planing, doing unnecessary work,..."; the narrowing of the same
day, "let the next batches only work on finishing ledger rows and pdf/a conversions (migrations)
until I say otherwise". Review 1815 holds the measurement, and its instruments are
`doc/reviews/1815/denominators.py` and `doc/reviews/1815/slots.py`. ADR 2370 is the ledger slot this
keeps. This ADR cites no clause and moves no ledger row.

## 1. What decides it

Review 1815 section 1. Batches 109–116 moved four ledger rows in 64 slots. From batch 117 on, the
moves were mostly intake: 35 of 51 were rows the owner's answers brought into scope. Nine of batch
108's twelve `partial` rows are still `partial`. Five of the seven leaf families still `partial` or
`reported` name a block no slot here can lift.

On the archive side, `pdf.js as-if-printed` at part 4 went from 703 to 795 over batches 109–119,
about seven documents a slot. Under the narrowing it moved −3 and then up to +4 over seven archive
slots. Round 1804's ranking puts about 280 of the refusal instances behind a profile's `stop`, the
fence or the press profile.

So the rounds are not stuck planning. Eighty of 110 built something. They are working remainders
that only decisions move, and 76.5 % of the round hours of batches 109–120 moved neither
denominator. The rules below put the frontier in front of whoever briefs a batch, and the decisions
in front of the owner.

## 2. The rules

**R1 — A narrowed brief opens on its frontier.** The brief's first section states two lists, each
by the command that prints it:

- the ledger rows whose block is in this tree: the `partial`, `departed` and `reported` rows less
  those whose note names an owner's answer, an unacquirable document, a platform limit or a CI run;
- the census's refusal reasons that a remedy can carry under the profiles' existing losses, ranked
  by documents. This is round 1804's ranking, re-run on HEAD with the pack's presence on the first
  line (ADR 2460).

The batch then briefs no more ledger or archive slots than the two lists hold objects. *Proposed,
until Q681:* the slots that are left go to what the owner's answer names. Until it comes, they go to
the frontier's largest object split in sequence (ADR 2372 rule 1), never to a remainder already
known to be blocked.

**R2 — A row evidenced only by CI is briefed on a run read.** A slot whose verdict is a CI job's
(§12.2's WinUI `window` job is the case) is briefed only once the CI run of the merge it waits on
has been read and has failed for a stated reason. A batch with no new run carries no such slot.
This turns the row once per CI run instead of once per batch. Q681 item 3 asks whether a run can
come before the merge.

**R3 — A row is briefed a fourth time only on a named owed sentence.** A row worked in three batches
without a status move (§10.7.4 was worked in six) is briefed again only when its note names the
sentence a fourth slot would discharge. A note rewritten without a status move is no longer than
the note it replaces, unless the slot's ADR says why the length is needed. The note says what is,
not what was tried (CLAUDE.md, "Where knowledge lives"). Review 1815 section 1.4 has the cost: a
78.7k-character note is read whole by every slot that touches the row.

**R4 — A continuing slot states its finish line as a status or a count.** Every slot whose object
an earlier brief carried (76 of 107 did) names the ledger status, or the census count, that it
moves when its contract is met. A step of an RFC is not a finish line. §13.2 finished eight rows in
one batch because each slot named them. XFA's five slots named RFC 0011 steps, and K.2's note was
rewritten four times. An XFA slot names the K.2 sentence it ends.

**R5 — A measure slot re-takes before it overturns.** When the measure, instruments or loop lanes
resume, a slot that would overturn an earlier round's figure first re-takes that figure on HEAD,
under the same load record the earlier one states. Eight of sixteen such slots in batches 109–119
overturned an earlier figure, most of them as the machine's load.

## 3. Rejected

- **A smaller batch.** The remainders stay the same and are carried more slowly.
- **An ADR quota.** The ADRs carry refusal and measurement arguments, and those are the project's
  memory.
- **Treating continuation as waste.** Continuation converges where R4's finish line exists.
- **A ledger-note length gate in `tools/conformance`.** R3's length rule is the orchestrator's check,
  read from `denominators.py`'s notes column. A gate would bind every row, including the rows whose
  notes rightly grow with a status move.
