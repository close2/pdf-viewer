# 1036 — An index, a tier, and a record

Session 1018. Status: **accepted**.

`doc/reviews/1012-where-the-effort-goes.md` measured what a round pays before it does any work and
what that payment buys. Three of its findings are decisions rather than observations, and this ADR
takes them so that a later round does not re-litigate any of the three. Nothing here deletes a
reason: every incident, argument and figure that was in the corpus is still in the corpus, and the
two things that moved are *where a round meets them* and *when a gate runs*.

## 1. The trap corpus is read through an index, not by opening a group

**`doc/traps/README.md` is the index and it is the standing read.** One line per trap, in two
halves: the **position** that puts a round in a position to spring it, and the rule. A round reads
the condition column against what it is about to do, and opens a group file only where a line bites.

The five group files are unchanged — 2,509 lines of incident, evidence and argument, and not a word
of it is touched. `doc/HANDOVER.md`'s own 41-row table, which named each trap and its group, moved
into the index and gained the condition column; HANDOVER now points at the index in one paragraph.

**Why an index and not a compaction.** The traps are cited by number and the citation counts are
extremely skewed: measured over `doc/history/`, `doc/adr/` and `doc/reviews/`, traps 13, 11, 9, 5, 1,
8 and 10 carry about 75% of every citation ever made and the other thirty-odd traps split the rest.
The reflex response to that is to cut the cold ones, and it is wrong twice over. First, `CLAUDE.md`
is explicit that a trap **is** an incident with a rule attached, that the history is the lesson, and
that deleting one deletes the reason anybody believes the rule. Second, the owner's age-normalised
count shows the raw figure is itself a trap: trap 37 has five citations because it is sixteen
sessions old and rates 31.2 per 100 sessions, level with trap 1's 31.6. So what was wrong was never
the volume — it was that a round read a **group** (up to 917 lines) to find the six lines that
applied to it. An index fixes the reading cost without touching the corpus.

`doc/reviews/1018-what-retiring-a-trap-would-cost.md` evaluates the sixteen genuinely cold traps one
by one against retire/merge/demote/keep. It is a proposal for the owner and this ADR enacts none of
it; the index covers every trap including any the owner later retires.

**The habits were measured and are *not* getting an index.** The same skew does not hold: all six
files in `doc/habits/` are cited between two and five times across `doc/history/`, `doc/adr/` and
`doc/reviews/`, which is flat rather than skewed, and there are no numbers to cite by — a habit is
cited by its file. The ~202 habits inside those files are already one-line-led bullets, so a file is
its own index, and `doc/habits.md` (45 lines) plus HANDOVER's table already route a round to the
right one. An index of every bullet would add about 200 lines to the standing read, which is exactly
the "compaction that grows the corpus" the review named in ADR 0983. Measured, decided against.

## 2. `doc/todo/02` §2's gate sequence is in three tiers

The sequence is 39 lines. Over sessions 912–1011, classified in review 1012 §3: **25 of them caught
no defect their own round introduced**, ~17 of the ~18 self-catches came off `cargo fmt`, the two
`clippy` lines, `cargo nextest` and `cargo test -p conformance`, and the ~25 minutes of corpus-scale
walking produced **one** self-catch against **fourteen** false alarms. `foreign_corpus` is the worst
single line: zero self-catches, five false failures, 76–214 s and 6.70 GiB, with session 999's record
saying it cannot be run beside another round's copy of itself.

- **Tier 1 — every round.** `fmt`, both `clippy` lines, `nextest`, `--doc`, the `fuzz/` pair, and
  `cargo test -p conformance`. It is where the catching happens and it is about a tenth of the cost.
- **Tier 2 — the round that touched the subsystem.** The change → gate map names which. A gate can
  only see what its own crate does, so a round that did not touch what a line walks is paying for a
  walk that cannot move.
- **Tier 3 — the merge, on `main`, in full.** All the corpus-scale walks. This is where their catches
  actually happened: session 1005's ten converter-fixture failures and session 985's `archive_corpus`
  signature defect were both found at a merge, not in a round.

**No gate is deleted and no coverage is lost.** Rule 5 — a merge runs everything, always — is what
makes the demotion safe: every walk still runs over every change, once per batch instead of once per
round. Rule 4's fifth round still runs all three tiers, and a round whose *subject* is what a walk
asserts about (the writers, the validator, `pdf-vfs`) runs that walk in tier 2. The four trap-10
`--bins` lines are marked as what they are — prerequisites of the lines under them, not gates — and
travel with them.

Rule 3 is the one that changes visibly: a pixel round runs tier 2's change detectors
(`pdf-model --test corpus`, `raster_golden`) and says in its record that it moved a pixel, and the
`oracle` verdict lands at the merge. **Trap 1 is not relaxed by any of this** and never was
satisfiable by a gate: render the page and look.

## 3. `doc/history/` is a record, and the quotation sweep reports it apart

`--bin quotations` walked 565 history files on every run and counted their quotations into the one
figure a round copies forward. `CLAUDE.md` forbids rewriting a record for tidiness; it does not
exempt a misquotation, which is a wrong claim about the standard wherever it is written. Both of the
obvious answers are therefore wrong: **skipping** `doc/history/` (adding it to `prose::NOT_READ`,
which is what review 1012 §6 proposed) loses a real finding, and **counting** it makes the live
figure unmovable, because a population of records that only grows can never be read to zero.

So the sweep now reports three populations: live documents, records, and the ledger's notes.
`prose::RECORDS` is `history` and `history.md`, matched by path component; `prose::is_a_record` is
the predicate and `a_history_file_is_a_record_and_a_todo_file_is_not` pins it. `doc/adr/` and
`doc/reviews/` are deliberately **not** records for this purpose — they are read, cited from compiled
code, and opened to settle questions, so a misquotation in one misleads a round at work.

Measured on this tree: the figure a round is asked to move falls from 51 diverging to **38**, with
**13** in the records — and the records line says in the output that it is corrected only where the
quotation itself is wrong.

## 4. What this cost and what it bought

The standing read — `CLAUDE.md`, `doc/HANDOVER.md`, `doc/todo/README.md`, `doc/todo/02-every-round.md`
and `doc/environment.md` — is 1,951 lines before and after: HANDOVER lost 60 and §2's three tiers,
with the one sentence per tier saying why, gained 60. The trap index adds 84. What comes off is the
**group file**: a round that writes Rust at all opened `instruments-and-reports.md` (917 lines) for
trap 7 alone, and no longer does. For that round the standing read goes from 2,868 lines to 2,035,
and it is now the same 2,035 whatever the round is about.

The gate saving is ~25 minutes of machine time per round and, worth more, the fourteen false alarms
that each cost a round a diagnosis.

**The precedent, which is the part a later round must not undo:** the corpus grew 15% over 131
sessions and neither named compaction removed a line, because both rewrote prose instead of changing
how it is reached. Indexing and tiering change *when* something is read and *when* something is run.
Neither is licence to delete a reason, and neither is a reason to stop writing one down.
