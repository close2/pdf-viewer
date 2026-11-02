# Q757 — May `state-every-loss` and `no-data-loss` take `recovered-structure`, so that A727's `page-not-held` can reach them?

Source: round 1881 (slot 1 of batch 129), building A727 (ADR 2596).

## The question

A727 accepts the named loss `page-not-held` in `as-if-printed`, `state-every-loss` and `no-data-loss`,
stated on the appended page under the last two. But `page-not-held` is asked only of a source the
archive rebuilds, and a source is rebuilt only under the loss `recovered-structure` (ADR 2502). Of the
shipped profiles only `as-if-printed` and `as-shown` take `recovered-structure`; `state-every-loss` and
`no-data-loss` take none, so `page-not-held` can never reach them: a document such as `A Defence of the
New Tenseless Theory of Time.pdf` is refused by them for its structure before the count is read.

Two readings of A727:

1. **They should take `recovered-structure` as well**, for the structure a reader draws the pages
   from (a catalog and one page tree node listing the pages found, each page the producer's), with
   `page-not-held = "append-statement"` so the appended page lists the root's count and the count
   carried. This widens both profiles past what they accept today: every document this viewer
   recovers would convert under them, not only the ones that hold fewer pages than they state.
2. **A727 means the loss is accepted only where the rebuild already runs**: the two profiles keep
   their refusal of a recovered structure, and `page-not-held` stays an `as-if-printed` loss.

## What is built, and what is not

Built: the loss, `[profile] page-not-held = "discard"` (taken; `as-if-printed` carries it) and
`= "append-statement"` (taken and listed on the appended page, with the two counts and the clause
that states the count). Not built: any line in `state-every-loss.toml` or `no-data-loss.toml`.

## Recommendation

Reading 1, limited by the profiles' own sentences: both say every loss is stated, and a recovered
structure stated on the page meets that; `no-data-loss` would take `recovered-structure` only with
`page-not-held = "append-statement"`, since the page not held is data the file names and does not
hold. The cost is one ADR and two profile lines per profile, and a census to price what moves.
