# Habits these sessions earned

Status: **standing** — method, not code. `doc/traps/` is the code half, five files grouped by
what a round is doing; this is the index of the six habit files, one per kind of work.
Read by: whoever is about to read a clause, judge against another renderer, write a gate, correct
a ledger row, or take a measurement — which is most rounds, **one file at a time**.

Each was paid for once. Traps are about code; these are about how to work. Every one keeps the
anchor that makes it checkable.

**This file is an index and nothing else** (ADR 0983). It was the largest file in this corpus
sitting behind one entry in `doc/HANDOVER.md`, opened one section at a time, and `tools/round.sh`
could only name the whole of it. The six headings below are kept as headings so that every citation
of a section by name — `doc/habits.md` *Measuring*, its *ledger section* — still resolves in one
hop, which is ADR 0232 §2's rule.

### Reading the specification

`doc/habits/reading-the-specification.md`

### Judging against other implementations

`doc/habits/judging-against-other-implementations.md`

### Tests, gates and reports

`doc/habits/tests-gates-and-reports.md`

### The ledger, and claims about this tree

`doc/habits/the-ledger-and-claims-about-this-tree.md`

### Measuring

`doc/habits/measuring.md`

### Code, bounds and dependencies

`doc/habits/code-bounds-and-dependencies.md`

**One thing the split made visible and did not fix**: four of *Measuring*'s habits are about a
ledger row's reason going stale — the `/CheckSum` entry deferred for cost, the fourth sweep shape,
sweeping for the reason's shape, and an `inapplicable` row waiting for a capability — and their
subject is the file above them rather than the one they are in. They were at the end of a section
nobody could see the end of; a round taking them across owes nothing but the move (ADR 0983).
