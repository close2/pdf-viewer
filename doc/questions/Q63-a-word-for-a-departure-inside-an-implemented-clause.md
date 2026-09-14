# Q63 — A status for one decided sentence inside an otherwise-implemented clause

Source: session 1017 (ADR 1035 §3), which read every `partial` row against the standard and found
twenty that are `partial` *correctly* under the ledger's own vocabulary, and wrongly under what a
reader of the ledger takes `partial` to mean.

## The fact

Twenty rows each record one `shall` addressed to this program that the project decided against,
with the cost measured and written down: §7.5.5's `/Size` (66 documents would be refused), §7.4.8's
`/ColorTransform`, §10.7.4's five scan-conversion departures, §12.11.6's refused "shall not
continue", and the rest ADR 1035 §3 names. Every other requirement of each clause is executed.

The ledger has `out-of-scope` and `inapplicable`, and both say "decided" for a **whole clause**.
Nothing says it for **one sentence** inside a clause that is otherwise implemented. So each of the
twenty wears `partial` — the same word as a row nobody has finished — and every count of "real
debt" this project prints includes them. ADR 1035 forbids re-statusing a departure to `implemented`
meanwhile, because that would hide the sentence.

## What is asked

Whether the status vocabulary gains a word for this — something like `departed`, meaning *every
requirement of this clause is executed except the one the note names, which was decided against
with its cost recorded* — or whether the twenty stay `partial` and the counts stay as they are.

The cost of a new word: `tools/conformance/src/ledger.rs`'s status enum, the aggregate rule
(ADR 1035 §5 — a `departed` row owes nothing, so a parent above it owes nothing on its account),
`tools/state.sh`'s counts, and the header of `ledger.toml`. Perhaps half a round. The cost of not
having it: every "how much is left" figure is twenty too high, permanently, and a round briefed on
`partial` rows can be handed one of these and spend its slot discovering it was decided in
session 400.

Recommendation: add the word. Twenty rows is enough to be a category and not an exception.
