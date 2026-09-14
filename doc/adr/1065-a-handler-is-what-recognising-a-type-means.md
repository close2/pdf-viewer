# ADR 1065 — A handler is what recognising a type means, and the next sentence is the proof

## Status

Accepted, 2026-09-14. Session 1051. Moves §12.5.1's ledger row from `partial` to `implemented`.
Test: `pdf-model/tests/annotations.rs::every_annotation_type_table_171_defines_is_answered_by_its_own_clause`.
`§N` is ISO 32000-2 and nothing else.

## Context

§12.5.1's last paragraph has held its row `partial` since the row existed, on one sentence:

> A PDF processor shall provide annotation handlers for all of the conforming annotation types.

Read alone it is unboundable. "Handler" is defined nowhere else in the standard; the sentence
before it says only that "the behaviour of each annotation type **may** be implemented by a
software module called an annotation handler", which is a permission about *construction*. So a
reader can take the `shall` to mean anything from *recognise the name* to *play the movie*, and the
row's own note took the widest reading — it listed three subtypes as owed because `CLAUDE.md`
excludes clause 13, which makes the row a duplicate of §12.5.6.17's and §12.5.6.25's
`out-of-scope` and of §13's forty rows besides. It was also wrong about which three: `Screen`'s row
is `implemented` on its own reader-facing requirement, `3D` was never on the list, and `Sound`'s
playing is §13.3's.

## Decision

**The `shall` is about recognition, and the sentence immediately after it is what bounds it.** The
paragraph continues:

> The set of annotation types is extensible.

and

> An interactive PDF processor shall provide certain expected behaviour for all annotation types
> that it does not recognise

The two sentences are a pair covering the whole world of subtypes: one for the types a processor
recognises, one for the types it does not. A type therefore has a handler exactly when the
processor answers *from that type's own clause* rather than from the fallback — and what that
fallback is, Errata Collection 3's Issue #1 says outright, sending the second sentence to §12.5.5's
appearance streams and Table 167's bits 1 and 2 instead of to §12.5.2.

All twenty-eight of Table 171's types are on the first side of that line here.
`appearance::construct` carries one arm per subtype, each with that clause's own reading — a
construction where the clause states a shape, a refusal quoting the clause's own reason where it
does not — and `annotation::decided` draws any subtype's stored `/AP` `/N` before reaching it,
which is §12.5.5's own sentence about a processor with no native support. The catch-all arm is
reached by one input only: an annotation stating no `/Subtype` at all, which Table 166 makes
required.

## Consequences

**What this does not claim.** The five media subtypes — `Sound`, `Movie`, `Screen`, `3D`,
`RichMedia` — are recognised and drawn, and their *activation* is clause 13's, excluded by
`CLAUDE.md` principle 5 and carried by the `out-of-scope` rows that own it. That exclusion is
untouched; what changes is that it is recorded once, where the excluded clause is, instead of a
second time in a parent that cannot act on it. Nor does this row absorb its siblings: how well each
subtype is *drawn* is each §12.5.6.x row's, and §12.5.6.11 and §12.5.6.12 stay `reported` on their
own clauses' silences.

**What would reopen it.** A Table 171 that grows a type — the clause says the set is extensible, and
a future part could add a standard subtype — or an arm of `construct` deleted. Both are what the
test is for, and it keeps the control that must reach the catch-all so that a run in which nothing
does proves nothing.
