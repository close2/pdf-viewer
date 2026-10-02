# 1499 — A record states its gates

Session 1332. Status: accepted. Code: `tools/conformance/tests/records.rs`
(`every_record_since_the_rule_states_its_gates`, `the_gates_check_is_calibrated_against_planted_records`),
`tools/state.sh` (`records`). Prose: `doc/todo/02` section 6 and section 8 item 2.

## 1. The question

A round's report reaches the orchestrator and is gone; the record in `doc/history/` is the only
place the tree keeps what a round ran and what it exited with. Of the records since 1284, the
brief's own count found three of batch fifty saying "see the report" or nothing, and the count
below found more. A record that says "see the report" points at a text the tree does not hold.

## 2. The rule

**Every record from session 1327 on carries a paragraph opening `**Gates.**` that names at least
one gate with its exit status or pass count.** The shape is the one most records already used: of
the forty-three records 1284–1326, sixteen open such a paragraph (three of them to say "see the
report"), five use a `## Gates` heading, three `**Gates**`, `Gates:` or `**Tests and gates.**`, and
nineteen state no gates at all. One shape
is checkable by a line prefix; five are a list of exceptions (trap 25).

What the check reads: the paragraph from the `**Gates.**` line to the first blank line. It is
*stated* when it contains a figure — a word that, once ASCII punctuation is trimmed, is all digits
(`0`, `918`) or a count over a total (`39/39`) — and does not say "see the report", which is
refused by name whatever figures sit beside it. It sees a figure, not that the figure belongs to a
gate: "(6 crates)" counts. That limit is stated rather than closed, because a parser for a gate's
name would be a list of gate names, which goes stale with every new gate. Planted records of each
verdict (stated, see the report, no figure, a `×` multiplier mistaken for a figure, another shape,
absent, a mid-line `**Gates.**`) calibrate it.

## 3. What it does not do

Records before 1327 are records, and `CLAUDE.md` says rewriting one is the single thing a record
may not have done to it; so they are printed with their verdict — `tools/state.sh records` lists
every record since 1284 that does not state its gates — and never failed. The rule binds from the
batch that made it, which every brief of that batch already stated.
