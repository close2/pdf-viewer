# 1599 — A heading's status is derived from its leaves, and never hand-written

Session 1381. Status: **accepted**. Builds on ADR 1035 (a status is a claim about a residue,
`Ledger::is_aggregate`) and ADR 1535 (a heading takes its subclauses' status); amends neither.
Context: `doc/PLAN.md` §5a; `tools/conformance/src/ledger.rs` (`Ledger::derived_status`,
`Ledger::derive_aggregates`, `Problem::AggregateStatus`, `Problem::AggregateNote`);
`tools/conformance/src/frontier.rs` (`Disagreement::Aggregate`); `doc/todo/65`'s aggregate list.

## The question

ADR 1035 states the rule an aggregate follows: a heading stays `partial` while any row beneath it
owes, and `Problem::AggregateWithoutDebt` catches the moment after its last child settles. Nothing
caught the other direction. A heading's status was still written by hand, so it could sit settled
over a child that owes, or carry a reason a child's note already gives. Should the checker derive
it instead?

## What the tree says

Seven rows have a row beneath them that owes: §7.4, §7.6, §7.6.5, §12.8, §12.8.3, §12.8.3.4 and
§12.10. In `doc/md/`, each of the seven is followed directly by its own `.1` subclause, so none
prints text of its own and none can owe anything for itself. Six were written `partial` or
`reported`. **§7.4 was `implemented` over §7.4.7 and §7.4.9, both `partial`.** That is exactly the
drift ADR 1035's rule forbids, and no instrument saw it, because `is_aggregate` holds a settled
row to be no aggregate at all. Round 1369's §12.1 was the reverse error, a row counted as debt of
its own. Both came from the same thing: a status that is a consequence of other rows was written
down as if someone had read it.

## Decision: derived

**A row with an owing leaf beneath it takes its status from its leaves.** A leaf is a row with no
row below it, and only leaves are read against a clause. If every leaf wears the same owing
status, the heading wears it too: §7.6.5's three subclauses are `reported`, so §7.6.5 is
`reported`. If the leaves differ, the heading is `partial`. `Ledger::derived_status` is the rule.
`Problem::AggregateStatus` fails `cargo test -p conformance` and names the row on a mismatch.
`cargo run -p conformance --bin ledger -- --write` sets it, and the counting run shows it as a
difference from the generated form. **The note of such a row opens "Aggregate of the rows below"**
(`AGGREGATE_NOTE`, `Problem::AggregateNote`) and says nothing a child says: a repeated reason
drifts the moment the child moves. **`doc/todo/65`'s aggregate list is computed.** The frontier
gate requires it to be exactly the rows `is_aggregate` names, and fails on an aggregate placed as
a bullet or a bullet placed as an aggregate. §7.6.5 and §12.10 left their buckets' bullet heads on
this rule, and their owing children stay there.

**What is not derived, and why.** Suppose every leaf under a heading has settled. Which settled
word the heading then takes is a reading of the family: `implemented`, or the one exclusion every
child rests on. So it stays hand-written, and `AggregateWithoutDebt` still stops it owing. A
heading that printed a requirement of its own *and* sat over an owing child would wear the
derived word, while its note's own reading follows the opening phrase. No such row exists today.

## What it costs

§7.4 moves `implemented` → `partial` (ADR 1598), and the `partial` count rises by one row that is
no new work. The seven headings' notes lose every sentence a child already carried. What no child
carried was moved into the child it belongs to and was not dropped. A heading can no longer
carry a summary of its family, so a reader starts at the owing row's note, which is where the
reason was already argued.
