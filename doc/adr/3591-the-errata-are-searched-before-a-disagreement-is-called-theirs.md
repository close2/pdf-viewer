# 3591 — The errata are searched before a disagreement is called another implementation's

Session 2222. Status: **accepted**. Context: ADR 3581 (step 5 of the owner's plan), ADR 3582
(veraPDF-library #1668).

## Decision

`doc/habits/judging-against-other-implementations.md` gains the habit: before a difference from
another implementation is called theirs, or drafted for `doc/upstream/`, the clause is searched in
`doc/errata/` (the site's page, `pdfa.md`, the issues and comments) and in `spec-errata check`'s
output. `doc/upstream/README.md` states the drafts' template, and its `Errata checked` line records
the search; where it finds an agreed correction the draft is not filed.

## Why

#1668 was filed against a validator that had followed an erratum published two years earlier. The
search costs a `grep`; the omission cost a census and an upstream issue.
