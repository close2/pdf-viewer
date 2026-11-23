# 3454 — Tier 1's throw ceiling is not lowered to a figure the load made

Session 2188. Status: **accepted**; nothing is built, and `tests/script_corpus.rs`'s held figures stay. Builds on
ADR 1625 (the Tier 1 column is a merge gate, ceilings and a floor) and ADR 1603 (the calculation order's one-second
budget). Amends ADR 1625's practice that a `ratchet:` line is always followed.

## Context

The train at `1d73fc0f6` printed `ratchet: threw fell from 9315 to 9285; lower its ceiling to it`, and failed. It
failed on the floor, not the ceiling: 21 096 runs against `HELD_RUNS` 21 101. The brief read the fall in throws as
thirty throws some commit removed.

## What the walk measured

Over the 356 documents the held figures were written from, in a private worktree, per commit:

| commit | runs | finished | threw | load |
|---|---|---|---|---|
| `498baf52` | 21 101 | 11 778 | 9 315 | light |
| `cca7b2660` | 21 081 | 11 763 | 9 310 | load average near 99 |
| `1d73fc0f6` | 21 086 | 11 763 | 9 315 | light |

- **Fifteen finished runs** went with `cca7b2660` and stay gone: the fields ADR 3453 lists as newly untaken are no
  longer committed, so their `/K` and `/V` scripts no longer run. That is the held documents' share of the runs, a
  correction's consequence.
- **The throws moved with the load and with no commit.** `cca7b2660` and `1d73fc0f6` read fields identically, and
  the five throws between them are all one document's: `safedocs/…/5712688.pdf`, whose 468 scripted fields run
  about 8 900 calculations that throw (`TFMC is not defined`). Single-threaded it threw 8 911. Every commit
  re-walks the calculation order under ADR 1603's one-second `MAX_SEQUENCE_TIME`, and on this document that walk
  costs a large part of the second, so a loaded machine stops it earlier and runs, and throws, fewer. The stop's
  own report is not visible here: the document fills the 256-sentence report cap first.

So no commit removed thirty throws; the train ran this document slower than the walk that set 9 315 did.
This round's whole-population walk at its head, under a load average between 50 and 220, ran 21 121 times and threw
9 315, and passed: 25 runs and 30 throws above the train's figures for the same document set.

## Decision

1. `HELD_THREW` stays 9 315. Lowered to 9 285, the next walk that is not slowed would fail with nothing changed.
2. `HELD_RUNS` stays 21 101, and the walk's `ratchet: the runs rose from 21101 to 21121` is not followed either:
   raised, the floor would sit at the top of a band the train measured 25 runs wide, and fail the next slow walk.
3. A `ratchet:` line is read against the commits in the range before it is followed (this ADR's reading).

## Cost, and what is left

Both figures still move with the machine through this one document, and a slow enough walk can fall below the
floor with nothing changed, as the train's did. Making the column independent of the machine — a calculation-order
budget the gate measures in work rather than time, or the documents a sequence budget stopped counted apart from the
floor and the ceilings — is the next round's, and is first in record 2188's list.
