# 2370 — Every batch's clause slot tries to move a ledger row, and an excluded row is re-read by class

Session 1768, a clause round beside batch 116. Status: accepted; the rule is the owner's of
2026-10-26, the method is review 1768's. ADR 2371 is unused.

## Context

The owner, 2026-10-26: "please use one round to evaluate if we should / could ledger rules from
inapplicable and out-of-scope into scope. Do the reasons still hold?" — and, in the same message,
"Please make sure, that in every ba[t]ch at least one round tries to improve the ledger." The second
sentence widens the owner's rule of 2026-10-08, that every batch implements or at least works on a
`partial` row, named in the brief's clause slot and reported in the commit's Ledger line.

Review 1768 read the 118 `inapplicable` and `out-of-scope` rows and found the reason decayed in 28 of
them, most of which no instrument reads: the `inapplicable` sweep (`--bin inapplicable`) covers only
its own status, and twenty-one §13.2 rows whose rules `pdf_model::media` executes still said "a
media engine" because they are `out-of-scope`. It moved none, because the two statuses its brief
allowed were false for every decayed row and each would have dragged two `implemented` headings to
`partial` through `Ledger::derived_status` (the review's section 1.4).

## Decision

1. **Every batch's clause slot tries to move a row**, and the commit's Ledger line names the row it
   tried and the status it reached, moved or not. The row is one of: a `partial` row toward
   `implemented`; a `departed` row re-read against its clause and its recorded cost; or a row from
   the newest scope review's candidate list (review 1768 section 4 until a later one replaces it),
   taken with its blocker cleared — a `Q` answered, or the build its row names. A slot that cannot
   move its row says which premise stopped it, and the next brief names the next row.
2. **A row leaves an exclusion at the status its work makes true, in the round that does the work.**
   A review classifies and recommends; the round that builds or reads moves the row, straight to
   `implemented` where the in-scope half is executed, or to `reported` where the program detects
   and says what it does not yet do. A row is not parked at `partial` or `unreviewed` where the
   word is false for it: the first claims an execution or a debt, the second fails `CitedButUnreviewed` for any
   clause the code cites, and either drags its headings.
3. **An excluded or inapplicable row is re-read by class.** A, the standard's own condition, checked
   against what this program now writes as well as what it draws; B, `CLAUDE.md`'s closed
   exclusion, with whether its trigger has been met by letter — a trigger is evidence for the
   owner, never a move; C, a reason about this program that has decayed, which is a move; D, a cost
   or scope judgement, which is a question; E, a reason unstated, which is a reading owed. A scope
   review is re-taken when its candidate list is spent, or when a round reports a trigger met.

## Consequences

The clause slot has a population that does not run dry when the `partial` rows do: 9 `partial` and
18 `departed` rows today (`tools/state.sh ledger`), and review 1768's ten candidates beside them.
The cost is a slot that cannot always succeed — a candidate blocked on a `Q` is skipped, not
forced — and a Ledger line that reports a failed try as plainly as a move. What the rule does not
do is let a round declare a clause out of scope: `CLAUDE.md`'s exclusions are revisited by
argument, and the argument is the owner's.
