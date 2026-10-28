# Q731 — Does A662's hardware lane still stand, now that its first item was measured and taken back?

Source: round 1865, the audit of every answered `A` file against the tree (ADR 2564).

## The question

`A662` adopted `Q662`'s recommendation in order: "the resident image texture first, yes; the hardware
lane after it, yes, for the moving view only". Item 1 was built, held byte-equal to the host, measured
and taken back because it moved the plan sheet's turn and step the wrong way (ADR 2512, `Status:
rejected`). Item 2 — every step of a gesture drawn by the hardware lane at sampled coverage, the compute
lane's exact frame drawn when the gesture ends, a fallback to the compute lane for any frame holding a
mark the hardware lane does not carry, and a gate of its own for §10.7.4 — has no code, no ADR, no
`doc/todo/` entry and no slot. Is item 2 still owed on its own, or did "after it" make it depend on
item 1 succeeding?

## Why it cannot be settled without the owner

The answer orders the two items, and the order can be read two ways: as a sequence (build 1, then
build 2, whatever 1 showed) or as a condition (2 rests on 1). No round may choose between those for the
owner, and the owner's word of 2026-10-27 — batches finish ledger rows and PDF/A conversions "until I
say otherwise" — also keeps a performance lane out of the batches until it is lifted.

## What the tree does meanwhile

Every frame of a gesture is drawn by the compute lane exactly as before; nothing is built and nothing
is refused. `A662` keeps its `Owes:` line, which this question does not edit.

## Recommendation

**Item 2 stands on its own, taken after the current narrowing is lifted.** The hardware lane's case in
RFC 0013 is the gesture's frame time, which item 1's texture residency was meant to help but does not
decide; ADR 2512's measure says nothing about rasterising a gesture's steps at sampled coverage. One
round of about 100 tool calls prices it first: a probe of the moving view on the plan sheet's pages,
both lanes, one clock hold, and the §10.7.4 gate's shape written before any lane is wired.
