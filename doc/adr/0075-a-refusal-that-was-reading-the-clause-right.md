# ADR 0075 — A refusal that was reading the clause right

Status: accepted, 2026-07-31.

## Context

Five sessions of specification-track work, and the corpus's own list had not been read in any of
them. Its annotation row held one entry that was not a malformed file and not a clause gap:

> Line: no appearance stream, and §12.5.6.7's `/LL` makes `/L` the leader lines' endpoints rather
> than the line's

Every word of that is true. Table 178 says exactly it: with `/LL` present, `/L` "shall represent
the endpoints of the leader lines rather than the endpoints of the line itself". The refusal drew
the wrong conclusion from a correct reading — because the *next* sentences state where the line
then is:

- `/LL` is "[t]he length of leader lines in default user space that extend from each endpoint of
  the line perpendicular to the line itself", and "[a] positive value shall mean that the leader
  lines appear in the direction that is clockwise when traversing the line from its starting
  point to its ending point (as specified by `L`); a negative value shall indicate the opposite
  direction".
- `/LLE` is the length of extensions "that extend from the line proper 180 degrees from the
  leader lines".
- `/LLO` is "the amount of empty space between the endpoints of the annotation and the beginning
  of the leader lines".

That is a complete construction: a perpendicular unit vector, three signed lengths, three
segments. Nothing is invented.

## Decision

**Draw it.** `appearance.rs` computes the clockwise perpendicular — `(dx, dy) → (dy, −dx)` in
PDF's y-up space, the same quarter turn §7.7.3.3's `/Rotate` asks about and the one a reader
working in a y-down raster gets backwards — offsets `/L` by `/LL` to place the line proper, and
draws each leader from `/LLO` to `/LL + /LLE`.

Two entries are still refused, and each states a different kind of nothing. Table 179's line
endings name shapes with no dimension — "[a] square", "[t]wo short lines meeting in an acute
angle", "approximately 30 degrees clockwise from perpendicular" — which was re-read this session
and still states no size. `/Cap` replicates `/Contents` as a caption, which needs a font no entry
of a line annotation supplies.

## What the fix is really about

Two lessons, and the second is the sharper one.

**A true observation is not a conclusion.** This is the same shape as the text-markup refusal ADR
0043 removed: a comment that correctly said the clause left something unstated, beside a clause
that stated it four sentences later. The tell is a refusal whose text is *about the clause* rather
than about a missing input.

**The refusal fired on the entry's presence, not on its value.** The corpus document it named,
`annotation-line-without-appearance.pdf`, writes `/LL 0` — which Table 178 defines as "no leader
lines". So a document asking for nothing was declined for asking. This project's habits already
carry "a presence condition is not a restriction on meaning" for Table 115's `/CIDToGIDMap`; this
is the same error in the other direction, and the two together suggest the general rule: **read
what an entry's *value* means before branching on whether it is there.**

## Consequences

- **The corpus ratchet moves for the first time in six sessions: 90 → 89 documents drawing
  incompletely**, and the oracle's agreeing set gains that page — 836 → 837 of the pages we call
  complete agree with the reference consensus.
- Three tests pin the geometry, one per sentence of the clause: which side a positive `/LL` puts
  the line on, that a negative one reverses it, and that `/LLO` and `/LLE` bound the leader. Each
  was confirmed to fail on the reading it excludes.

## And two ledger rows that claimed too little

Reading the corpus list against the tree also found §8.4.5's row still saying `/Font` "is the one
owed entry … reported rather than passed over". It is implemented: the font cache is keyed by
object identity as well as by name, and `extgstate.pdf` draws with nothing reported. That makes
**three families in four sessions** whose rows understated the code (§14.7's six, §8.4.5's one),
which is worth stating as a rule rather than as three incidents: a `silent` or `partial` row is a
*lower* bound on what exists, and only reading the family against the code corrects it.
