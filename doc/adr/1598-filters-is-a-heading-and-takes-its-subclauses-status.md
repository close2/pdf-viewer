# 1598 — §7.4 is a heading, and it is `partial` while §7.4.7 and §7.4.9 are

Session 1381. Status: **accepted**. Moves §7.4 from `implemented` to `partial` under ADRs 1035,
1535 and 1599; amends none of them.
Context: `doc/PLAN.md` §5a's `partial` row ("A head is `partial` while a row beneath it owes");
ISO 32000-2 §7.4, §7.4.7, §7.4.9.

## The question

§7.4 was `implemented`. Its note described Table 6's ten filters, the refusal vocabulary and the
resumable decoders, all of which are §7.4.1's to §7.4.10's subjects. Two of its subclauses are
`partial`: §7.4.7 for the extended-template patch the JBIG2 fork has not taken (ADR 1459), and
§7.4.9 for the JPEG 2000 spaces whose texts are not held (ADR 1574).

## What the clause says

Nothing. In `doc/md/`, "7.4 Filters" is followed directly by "7.4.1 General", so the heading
prints no sentence of its own and states no requirement. ADR 1535 then decides its status: a
heading takes its subclauses' status. ADR 1035 decides which of theirs it takes: `partial` while
any of them owes. `implemented` claimed that every requirement under the heading is executed, and
§7.4.7's and §7.4.9's own notes say two are not.

## Decision

§7.4 is `partial`, as `Ledger::derived_status` derives it (ADR 1599). Its note is "Aggregate of
the rows below". What it described is carried by the subclause rows it belongs to: what they
already said was dropped, and the rest was moved into them. It joins `doc/todo/65`'s aggregate
list and is not a piece of work: it moves when §7.4.7 and §7.4.9 do.

## What it costs

Nothing a reader can see. No code changes, and no page draws differently. The `partial` count
rises by one heading, which `Ledger::owing` does not count as debt.
