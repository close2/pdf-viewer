# ADR 1261 — An aggregate whose children are all executed states nothing of its own

## Status

Accepted, 2026-09-22. Binds `doc/conformance/ledger.toml`'s §8.11 family and `doc/todo/65`.

`§N` is ISO 32000-2 and nothing else.

## Context

Four rows — §8.11, §8.11.1, §8.11.4 and §8.11.4.1 — were `partial`, and each closed on the same
sentence: usage application dictionaries for the `Print` and `Export` events, "whose changes
persist only for the duration of an operation this program does not perform". ADR 1173 built the
events and ADR 1174 named the operations that supply them, so the sentence stopped being true, and
three of the four rows were corrected above it while the sentence itself stayed at the end. The
fourth, §8.11.4.1, carried it as its own residue, and the other three then rested on §8.11.4.1.

That is a ring: an aggregate held open by an aggregate held open by a retired sentence.

## Decision

**§8.11.4.1 is `implemented`, and §8.11, §8.11.1 and §8.11.4 follow it.**

The clause is read rather than inferred. §8.11.4.1 is one paragraph and a list of pointers: "A PDF
document containing optional content may specify the default states for the optional content
groups in the document and indicate which external factors shall be used to alter the states. The
following subclauses describe the PDF structures that are used to specify this information." Its
`may` is a permission addressed to a document. Its `shall` is about what a document's own entries
indicate, and the two structures that carry it out are §8.11.4.4's usage dictionaries and
§8.11.4.5's determination — both `implemented`, as are §8.11.4.2 and §8.11.4.3. So the clause
states no requirement its children do not, and no requirement of its own is unmet.

**An aggregate row's status is its children's, and nothing else.** Where every child is
`implemented`, the aggregate is; where one is not, the aggregate's note names which. A row that
aggregates may not be held `partial` by a sentence about the tree, only by a child.

The retired sentence is deleted rather than annotated, in all four rows and in §8.11.4.5's, which
carried it twice beside its own correction (`CLAUDE.md`, "Where knowledge lives").

## Consequences

`doc/todo/65`'s aggregate list loses all four; the frontier gate
(`cargo test -p conformance --test conformance the_frontier_map`) is what says so.

The decay to watch is the mirror of the one that produced this: a child moving *off*
`implemented` leaves four aggregates claiming more than the family does. That is what the
aggregate notes now say in one sentence each, so a round moving a child knows which rows follow.

Supersedes nothing. Amends the status claims of ADRs 1106, 1173 and 1174's rows, not their
readings.
