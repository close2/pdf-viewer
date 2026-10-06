# 1573 — Clause 12's introduction states no requirement, and it is not an aggregate

Session 1369. Status: **accepted**. Moves §12.1 from `partial` to `implemented` under ADRs 1035 and
1535; amends neither.
Context: `doc/PLAN.md` §5a's `implemented` and `partial`; ADR 1035 (`Ledger::is_aggregate`), ADR
1535 (a clause stating no requirement of its own is a heading); ISO 32000-2 §7.1, §9.1, §12.1.

## The question

§12.1 was `partial`, and its note read it as clause 12's own map: every row of clause 12 is one of
the settled statuses or `partial`/`reported`, so the map was partial because the clause was. The
frontier map (`doc/todo/65`) listed it among the aggregate rows.

## What the clause and the rule say

The whole of §12.1 is one sentence and a list of what the subclauses describe: "This clause
describes the PDF features that allow a user to interact with a document on the screen", then one
bullet per subclause, §12.2 to §12.11. It states no `shall`, no `should` and no permission.

ADR 1035's rule is mechanical: a row is an aggregate when a row whose clause number it is a strict
ancestor of still owes something. §12.1 is a sibling of §12.2 to §12.11, not their ancestor, and no
§12.1.x row exists, so `Ledger::is_aggregate` is false for it and the ledger counted it as a row with
debt of its own. ADR 1535 answers what such a row is: a clause that states no requirement of its
own is a heading, `implemented` vacuously or as its subclauses are. §7.1 and §9.1, the same
introductory maps for their clauses, are `implemented` while clause 7 carries `partial` rows.

## Decision

§12.1 is `implemented`, vacuously. What clause 12 still owes is in the rows that owe it —
§12.8.3.4.4 under §12.8, §12.8.3 and §12.8.3.4, and §12.10.2 under §12.10 — and each of those
heads stays `partial` while its child does, which is ADR 1035's rule working. `doc/todo/65`'s
aggregate paragraph no longer names §12.1.

## What it costs

Nothing a reader can see: no code changes and no page draws differently. The `partial` count falls
by one row that was never a piece of work.
