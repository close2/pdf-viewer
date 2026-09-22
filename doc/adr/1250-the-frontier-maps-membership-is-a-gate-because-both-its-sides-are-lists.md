# ADR 1250 — The frontier map's membership is a gate, because both its sides are lists

## Status

Accepted, 2026-09-22. Session 1206. Binds `tools/conformance` and `doc/todo/65`. Carries out
ADR 1237's finding rather than amending it.

`§N` is ISO 32000-2 and nothing else.

## Context

ADR 1237 re-derived `doc/todo/65` by reading all 80 open notes and recorded what the drift had cost:
four `implemented` rows in the map's aggregate list, six `departed` rows in its buckets as though
they were owed work, and a closing section naming moves rather than positions. It ended by stating
the rule — *re-derive membership from `grep 'status ='` and assert the two lists agree* — and that
assertion is what nothing did. Six rounds and one batch later the map had drifted again in both
directions, which is the whole argument for making it a gate rather than a habit.

`doc/habits/the-ledger-and-claims-about-this-tree.md` says why a habit was never going to hold it:
*nothing fires when a stated blocker expires*. A row closing is a stated blocker expiring, and the
round that closes it is reading its clause, not the map.

## Decision

`conformance::frontier` derives the map's membership from the document and the ledger's open
membership from the rows, and `cargo test -p conformance` fails on any disagreement — a placed
clause whose row is not `partial` or `reported`, a placed clause with no row at all, an open row
placed nowhere, and a clause placed twice.

**Two places in the document count as a placement, and the prose around them does not.** A bullet's
head is the run of clause numbers before its first EM DASH; the aggregate list is the lines under
the *Aggregate rows* heading that open with a SECTION SIGN. Everything else is prose *about* a row,
and a bullet's prose names its neighbours constantly — "the row moves with the family above it",
"§11.4.6's NOTE 6", "the same quantity in §11.4.3's own sentence". Reading after the dash would have
every bullet claim the rows it merely mentions, and the map would pass while placing nothing.

The cost of that rule is stated rather than hidden: a row *described* in a paragraph of prose is not
placed, and the gate says so. §12.7.6.2 was exactly that — three sentences under bucket 5 saying
what it waits on, and no bucket holding it — and it is a bullet now.

**The population is `partial` and `reported`, which is what the map's own opening lines claim.**
`silent` and `unreviewed` owe something too and are deliberately outside: a `silent` row has nobody's
reading behind it to bucket, and `unreviewed` is the ledger's initial state.

## What its first run found

Four, all of them the drift ADR 1237 priced, none of them visible to any other instrument:

- §11.3.6 and §11.4.8 — `implemented`, and still in a bucket and in the aggregate list.
- §11.4.4 — `partial`, and named only in a sentence explaining what §11.4.3 defers to. Its residue
  is §11.4.6's own (the element whose one alpha is the product of shape and opacity), so the two
  rows share a bullet now.
- §12.7.6.2 — `partial`, described in prose and placed nowhere.

## Consequences

- A round that closes a row now has to take it out of the map, and a round that opens one has to
  bucket it. That is the point, and it is the cost: the gate is red until the map is edited.
- The gate cannot say a row is in the *right* bucket, only that it is in one. Which bucket is a
  reading of the note, which is ADR 1237's rule and stays a person's.
- The map's prose is unconstrained, which is deliberate: a bullet that explains a neighbour is
  better prose than one that does not, and the parser must not punish it.
