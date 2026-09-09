# 0926 — PDF/A is on, and part 4 is certified first

Session 941. Status: **accepted**. Recorded under the questions-directory rule: when an `A` file
appears, the round that acts on it records the decision in an ADR — the `A` file is the owner's
word, and the ADR is what the tree did about it.

## The answers this records

Four arrived together on 2026-09-08/09, and one governs the rest.

**`A46` turns PDF/A on.** The hold that `A15` and `A17` had placed on the feature is over, RFC 0006
is ratified, the validator comes first, and its shape is confirmed: one requirement table over
ISO 19005-2 and ISO 19005-4 with the levels as an applicability column rather than as separate
implementations. The instruction that changes what happens next is the last one — **finish and
certify PDF/A-4 first.**

**`A20` confirms the not-checked verdict as built**, and restates the rule that made it necessary:
a check is never implemented from a secondary source. `crates/pdf-archive/src/report.rs` already
carries `Outcome::Unchecked` and excludes processor obligations from the coverage line
(ADR 0923); this answer is what makes that shape settled rather than provisional.

**`A22` settles the names**: `quorra-transform archive` and `quorra-retrieve archive-check`, the
recommended pair, with no preference expressed against them.

**`A49` is void.** It asked whether to buy ISO 32000-1:2008, PDF/A-2's base document, and the owner
obtained Adobe's free copy the day after the question was written.

## What the tree does about them

**Ordering.** Every agent working this tranche is now told that a PDF/A-4 miss outranks a PDF/A-2
one. That is the only operational change `A46` makes, and it is a real one: the two parts share
most of their predicates, so work aimed at part 4 mostly helps part 2 anyway, but where they
diverge the tie is broken by the owner rather than by whichever file a sweep happens to list next.

**"Certified" is not a word this project may use loosely.** `A46` asks for PDF/A-4 to be *finished
and certified*, and what the tree can honestly claim is bounded by three numbers the sweep already
prints: how many of part 4's requirements are checked, how many stay `Unchecked` and why, and
whether `over` is zero. A validator with a truthful `Unchecked` list is finished in the sense this
project means; a *conforming-validator* claim with no gaps at all is a different and larger thing,
and `A51` says explicitly that it is a new question to be asked if it is ever wanted. So the target
here is the first, and the second is not smuggled in with it.

**`A20` moves nothing and that is the finding.** The row-level `Unchecked` reason, the per-target
report and the coverage line that excludes processor obligations were built before the answer
arrived and are confirmed by it. What the answer adds is a prohibition with teeth: an `Unchecked`
reason may never be weakened to make a row look better, because the reason *is* the report. Three
agent briefs in this session carry that sentence.

**`A22` costs nothing now and would have cost a rename later**, which is exactly why `Q22` said it
was cheap to answer early. `quorra-retrieve archive-check` already exists under that name.

**`A49` retires rather than answers.** `doc/questions/Q49` keeps the recommendation it carried
before the facts moved — buy it if and when PDF/A-2 becomes a target in its own right — because a
reader who finds only the conclusion learns less than one who finds what the reasoning was. The
work it leaves behind needs no owner: `doc/md/ISO_32000-1_2008.md` is prepared, and part 2's rows
can now be read against the edition part 2 actually names instead of against ISO 32000-2.
`doc/pdf-a-conversion-limits.md` §1.2 has stopped being a limitation about evidence.

## The one thing worth warning the next reader about

**RFC 0006 is ratified and its §0 is still true.** It was written from free previews, before the
owner bought the parts, and several of its readings were overturned by the purchased text — §5.1
on Level A and §5.7 on font substitution most sharply. Ratifying a document does not retroactively
give its author the standard. A claim in RFC 0006 about what ISO 19005 requires still needs
checking against `doc/pdfa/`, and `doc/pdf-a-conversion-limits.md` is the document written *from*
the standard rather than around it. The RFC's own status block now says so.
