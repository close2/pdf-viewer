# 1547 — A ledger note states what is

Session 1356. Status: **accepted**. Extends ADR 1023's rule to `doc/conformance/ledger.toml`'s
notes, on ADR 0281's ground; amends nothing else.
Context: `CLAUDE.md` *Where knowledge lives* (the comment rule and the counted-fact rule);
`doc/PLAN.md` §5a (what a status is and who sets it); ADRs 0281, 1023, 0974; the ledger's own
header comment; `tools/conformance/tests/ledger_notes.rs`.

## The question

ADR 1023 states the rule — a comment states why the code is as it is today and cites the ADR that
argued it, and a retired sentence is deleted rather than annotated — and says it binds comments and
the four navigational documents and "nothing else", naming the exclusions: `doc/adr/`,
`doc/history/` and `doc/reviews/` are records, and a trap or a habit *is* an incident with a rule
attached. The ledger's notes are on neither list. They carried 1914 session ordinals by the count
in the test below ("until the seven-hundred-and-thirty-ninth", "Re-read on 2026-09-16 and holds").
So either the ledger is a record ADR 1023 forgot to name, or it is a live document ADR 1023 did not
reach.

## Decision: bound

**The ledger is a live document.** Its own header defines a status as "a claim a person makes
after reading the clause against this code", and `doc/PLAN.md` §5a makes the note what says which
requirements are executed, which are not and what is reported. That is the current state with its reasoning — the thing a round reads before it touches a
clause, and the thing `CLAUDE.md`'s two denominators measure coverage by. A record is read to learn
what happened; a note is read to learn what is. Both of ADR 1023's exclusions fail on the ledger:

- **Not a record.** A record is never rewritten for tidiness; a note is rewritten every time its
  status changes, which is the whole of `--bin ledger`'s contract (it preserves statuses and notes
  only so that a person can change them). A document whose purpose is to be corrected is not one
  whose chronology is the content.
- **Not an incident.** A trap keeps its history because the history is why anybody believes the
  rule. A note's argument is the clause, the code and the test, all of which can be re-read today;
  the session in which a sentence was first wrong adds nothing a reader can check.

ADR 0281 decides the rest on its own terms: a session number and a re-reading date are counted
facts — "counts, rates, gate results, 'N of M', session numbers, dates" — and its one carve-out, a
number that is the *anchor of a lesson*, does not cover "since the six-hundred-and-forty-second":
the lesson, where there is one, is in the ADR the note cites. The cost ADR 1023 measured is the
ledger's at its sharpest: §12.4.4.1's note was 10 987 characters, and its first account of the
transition styles was a sentence retired three times in the same paragraph.

**What a note keeps**: the reading of the clause, the code and the test that hold it, every
verbatim quotation the current reading rests on, the ADRs that argued the present state, open
questions, errata as present facts, corpus witnesses beside the command that counts them, and a
choice recorded as a choice. **What it loses**: session ordinals, re-reading dates, "this row said
X until …", the tests a row used to cite, and the story of which sweep found a mistake. Where that
story is the reason the code is as it is, the reason stays and the ADR is cited.

## Applied to one family, and held for the rest

Clause 12 carried the most (635 of 1914). Its 130 rows that carried any — 124 rewritten by reading
each note against the rule, six re-read with the out-of-scope rows of ADR 1548 — now carry none,
and the clause-12 notes went from 771 945 characters to 579 081. Every quotation kept was
copied from the old note and checked against it; `cargo test -p conformance` ran after each chunk.
Two `Cited by` lines (`doc/todo/25`, `doc/todo/37`) named a ledger note whose only mention of them
was a retired sentence; they now name the comments that cite them.

`tools/conformance/tests/ledger_notes.rs` holds the remaining count by equality —
`ORDINALS_IN_THE_LEDGER`, printed with the families largest first — so a row may not add one and a
round that takes some out lowers the bound in the same pass. Clause 7 is next.

## What it costs

The ledger's sweeps that date a note by its ordinals (`overtaken`, `retired`) see less in clause 12,
because there is less chronology to read; that is the rule working, and `doc/adr/` and
`doc/history/` still date every decision. A note rewritten by a reader can lose an argument a
careless deletion would not notice; the equality bound is why the families come off one at a time,
each with its quotations checked, rather than in one pass.
