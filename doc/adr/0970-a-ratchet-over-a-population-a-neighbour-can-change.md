# 0970 — A ratchet over a population a neighbour can change, and it had already moved

Status: accepted. Session 963.
Context: `crates/viewer-core/tests/accessibility_census.rs`, `doc/todo/31-accessibility-host.md`,
`doc/traps/instruments-and-reports.md` trap 39, ADR 0962.
It **amends ADR 0962** by carrying that ADR's finding to a second instrument, where it lands on the
opposite side.

## What failed

`accessibility_census` asserts a ceiling on *pages with no `/StructParents` whose whole-tree
fallback answered nothing*: it was 56, the run found 60, exit 101. The four new ones are pages 4 to
7 of `icc_1_2001-12.pdf` — a profile specification downloaded into `doc/` the day before, for the
validator stream.

Session 960 attributed it exactly (move the file aside: 56 and exit 0; move it back: 60 and exit
101, nothing else changed) and left the shape to be argued, which is what this is.

## The finding is not the four pages

`population()` is a `read_dir` of `doc/`, and `.gitignore` excludes the specifications this project
has bought. So the population is **a function of what somebody else put on the disk**, and the
ceiling is a count over it.

**It had already moved, repeatedly, and nobody had noticed** — which is the part worth an ADR. Of
the sixty entries, **sixteen are gitignored `doc/` specifications**: `ISO_32000-2_sponsored_EC3.pdf`
pages 1002 and 1020, six pages across the four `ISO_TS_3200x` technical specifications,
`ISO-14289-2-2024-sponsored.pdf` page 50, and now four of `icc_1_2001-12.pdf`. Every one of those
arrived by download. The literal `56` was therefore never a measurement of this program's behaviour
over a fixed corpus; it was a measurement of this program over *whatever specifications had been
bought by the day somebody last ran the gate*, and it was silently re-baselined each time a round
raised it to make the gate green.

The failure was visible only now because a download finally pushed it over rather than being
absorbed.

## Why the existing guard did not catch it

The floors in the same function are guarded:

> Every floor here is a count over 988 documents […] Comparing a smaller population against these
> would fail for the one reason that is not a regression, so the floors are skipped and the skip
> says so

That argument is correct and it is **half of the argument**. A floor measured over a population is
wrong when the population *shrinks*; a ceiling measured over a population is wrong when it *grows*.
The guard was written by somebody thinking about a missing submodule — a real and common case — and
it covers only that direction. The ceilings ran unguarded in the direction that could actually move
them.

This is the same shape as trap 39, found in the same session: a check whose condition is wrong in a
way that produces no visible failure until much later, because the wrong direction is the quiet one.

## The decision

**The ceiling is held by name, not by count.** `NO_PARENT_KEY_SILENT` lists all sixty
`<document> p<page>` keys; the assertion is that no *entry* falls outside it.

The property that makes this the right instrument rather than a bigger literal is asymmetry that
matches the population's:

- a page that **joins** the class is not in the list, so it fails the run and is named — which is
  what a ratchet is for;
- a name that is merely **absent** proves nothing and fails nothing — so a fresh clone with none of
  `doc/`'s specifications, or a machine that has bought different ones, passes without the gate
  having been weakened for anybody.

A count cannot express that, because a count cannot tell a page that left from a page that was
never there. This is the shape the oracle's contradicted pages already use, and ADR 0962 is why: it
established that `doc/`'s gitignored PDFs are legitimately part of a population, so the answer is to
make the instrument tolerant of their coming and going rather than to remove them.

Calibrated both ways, per trap 13: with the list as measured the gate exits 0; with one name
(`comments.pdf p7`) deleted it fails, naming that page and its diagnosis.

## What was deliberately not done

**The other three ceilings keep their counts.** Two are 0 — and a ceiling of 0 already *is* a
by-name check, since any entry at all fails it and is printed. The third, `documents that would not
open`, is 2 and has the same exposure in principle; it is left as a count because nothing has moved
it, and converting it now would be a change with no finding under it. The comment on the changed
one says what the exposure is, so the next round that sees that number move has the argument to
hand.

**The floors are not touched.** They are guarded, correctly, for the direction that can hurt them.

**`doc/`'s specifications are not removed from the population.** They are the largest tagged
documents this tree has, two of them are the only things here that reach ADR 0325's node bound, and
ADR 0962 already ratified them as evidence. Removing them to make an instrument easier would be the
corpus going quiet.
