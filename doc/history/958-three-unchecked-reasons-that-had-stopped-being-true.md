# 958 — Three unchecked reasons that had stopped being true, and the first that was too optimistic

Date: 2026-09-11. ADR: 0956.
Files: `crates/pdf-archive/src/table/`, `crates/pdf-archive/tests/corpus.rs`.

The validator stream. It closed `metadata/xmp-character-data-only-in-simple-values`, split the
JPEG 2000 device-colour row in two because it was two rules under one identifier, and swept every
`Check::Unchecked` reason against what this project now holds.

**Every stale reason found before this round overstated a debt** — a row said a text was needed and
the text had arrived, or said a capability was missing and it had been built. This round found the
first one that erred the other way. `xmp-packets-describe-one-resource`'s reason called the missing
`rdf:about` *the only thing that says which resource a description is about*. That is an assumption
about the **serialisation**, ISO 16684-1 7.4, which lies past the preview's last page this project
holds and is never spelled there. The row is **further from closable than its own reason claimed**.

That is the finding, and it is a general one: a reason is a claim about the world and it decays in
both directions. A sweep that only looks for debts that have been paid will find every optimistic
reason still standing, and an optimistic reason is worse than a pessimistic one — it promises a
closure nobody can deliver, and it does so in the one place a future round goes looking for cheap
work.
