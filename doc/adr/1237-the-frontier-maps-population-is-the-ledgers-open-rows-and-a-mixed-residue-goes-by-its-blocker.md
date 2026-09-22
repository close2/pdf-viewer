# ADR 1237 — The frontier map's population is the ledger's open rows, and a mixed residue is placed by its blocker

## Status

Accepted, 2026-09-22. Session 1200. Binds `doc/todo/65`, and nothing else.

`§N` is ISO 32000-2 and nothing else.

## Context

`doc/todo/65` groups the ledger's unfinished rows by *why* each is unfinished, so that a brief can
pick the ones a round can actually take. Its header has always said the question is asked "of the
rows that are `partial` or `reported`", and its buckets have never held only those: `departed` rows
were carried in buckets 2, 3 and the not-owed list, and the file had grown a "what left this bucket"
section recording moves rather than stating positions.

Two things follow from that and both were visible on this re-derivation.

**A `departed` row is not open work.** `doc/HANDOVER.md` states the word: every requirement executed
except the one the note's first sentence names, with its ADR pricing it. That is a decision already
taken, and `tools/state.sh` counts it apart from `partial` for exactly that reason (ADR 1119, the
owner's word in `doc/questions/Q63`). A map of what is *left* that lists decided departures invites
a brief to re-open one, which is the shape `CLAUDE.md` calls revisiting by attrition.

**Several rows have a residue with more than one blocker.** §12.7.5.3 owes a file dialogue (a
surface) *and* reports a `RichText` field's formatting (an exclusion). §12.3.5.1 owes Table 158's
undivided window (a documented choice, ADR 0202) and `/Colors` (no `shall` at a processor at all).
§10.7.4 carries four departures §10.7.1's NOTE licenses *and* a clip region no backend's vocabulary
states. A row placed in two buckets is a row counted twice by whoever reads the map, and a row placed
by its first sentence is placed by whichever round wrote that sentence last.

## Decision

**1. The population is the ledger's `partial` and `reported` rows, and only those.** A `departed`
row is out of the map whatever it waits on; what it waits on belongs in its own note, which is where
`tools/state.sh departures` reads it from. Three departures are named in bucket prose because a
round steering the campaign would otherwise have to discover them — §12.7.8.3.4's unheld
ISO 19444-1 sections, and §8.7.4.5.7's and §8.7.4.5.8's one remaining patch branch — and a name in
prose is not a bullet: nothing in this map asks a round to take one.

**2. A row with a mixed residue goes in the bucket of the blocker that would have to move first.**
Not the largest part of the residue and not the first sentence of the note: the one a round would
hit. So §12.7.5.3 is host-UI, because the dialogue is what a round would build and the `RichText`
half is excluded outright; §12.3.5.1 is not-owed, because both of its halves are choices already
made; §10.7.4 is hard-rendering, because the clip region is a real unbuilt construction and the four
departures are priced choices beside it. Each bullet says the *other* half out loud so the placement
can be checked rather than trusted.

**3. The map states positions and not moves.** The "what left this bucket, and where it went"
section is deleted rather than carried forward. Every claim in it is recorded twice already — in the
deciding ADR (1191, 1185, 1197, 1121) and in the row's own note — and `CLAUDE.md`'s rule about the
four navigational documents applies here for the same reason it applies there: when a correction is
appended, the first thing a reader meets is the retired sentence. `doc/history/` keeps the
chronology.

## Consequences

- The map shrinks and stops disagreeing with `tools/state.sh ledger` about who is in it. A round can
  check membership with one `grep` over `status =` and the two lists agree by construction.
- A `departed` row's blocker now has exactly one home, its note, and one reader,
  `tools/state.sh departures`. If that turns out to hide something a campaign needs, the fix is a
  section in that instrument rather than a bullet here.
- Rule 2 is a judgement and it is meant to be re-taken: a bullet that names its other half is a
  bullet a later round can disagree with on the evidence rather than on the wording.
- **What this does not change**: the six reasons, the aggregate and not-owed sections, and the
  expired-premises section keep their meaning and their argument. The buckets are the content; the
  population rule is only about who is eligible for one.
