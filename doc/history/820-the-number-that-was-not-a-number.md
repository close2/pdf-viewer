# 820 — The number that was not a number, and the repair the tree refused

Finding: **the sixth blindness is closed** — `spec-errata renumbered` prints an erratum that
renumbers a table by striking its caption, which neither `check` nor `moved` can reach — and, on
the way to it, **`conformance::citation` turned out to be unable to read a table designation that
is not a `u16` at all**, so `Table Annex O.3`, `Table D.2` and `Table 125a` were no reference to
anything. The round's other half was declined: **the citations were not renumbered, because the
tree's standing answer says they must not be**, and the citations were checked and found right.

Date: 2026-08-29. Argued in ADR 0750. (0749 is a sibling round's; this number was taken one above
the tip on that reservation.)

Touched: `tools/conformance/src/citation.rs`, `tools/conformance/src/clause.rs`,
`tools/spec-errata/src/renumbered.rs` (new), `tools/spec-errata/src/lib.rs`,
`tools/spec-errata/src/main.rs`, `doc/conformance/ledger.toml` (§O, §O.2.1, §O.2.2),
`doc/errata-read.md` (one paragraph, the sixth blindness), `doc/todo/01-ledger-partial-rows.md`,
`doc/todo/02-every-round.md`, `doc/adr/0750-…`.

## The instruction the tree contradicted

The round was set to move the lines standing on Annex O's retired table numbers to the amended
ones. **`spec-errata moved` prints the refusal of that on every run** — the numbers are not changed
anywhere, because `doc/md/` is the published text every citation resolves against — and
`doc/errata-read.md` and the three Annex O ledger rows say the same thing in the same three parts.
ISO 32000-2 captions `Table Annex O.3` and has no `Table Annex O.1`; a tree citing the amended
designation would cite a caption no reader can find.

So nothing was renumbered. What *was* verified is that the citations are right on their own terms:
every place naming `Table Annex O.3` attributes one of that caption's five parameters to it and
every place naming `Table Annex O.4` one of its six, which is how the two tables divide Annex O's
eleven. The one line naming both divides them in one sentence.

The claim that decayed is a different one, and it was this round's own doing: three ledger rows
said **no instrument in this tree can see it**, and there is one now.

## What the predicate cost to build, and what running it taught

ADR 0746 wrote the shape down: a `StrikeOut` whose covered text is a table designation, paired with
a `Caret` whose contents are another. Two corrections came off the first run.

**The pairing is §12.5.6.2's `/IRT` group rather than the page.** Issue #124 alone puts four
strike-and-caret pairs on page 483 of ISO 32000-2, and pairing on the page would cross them.

**The shape alone is nine parts noise.** Grounded so that the struck text has to be a designation
the conversion actually *captions*, the predicate admits eleven annotations — and nine of them are
integers struck in body text: four array indices corrected to be zero-based, two NOTEs renumbered,
a font example, a function's domain, an LZW byte count. A bare `3` is a table designation and an
array index and a NOTE's number at once. The second grounding is what ranks the report: **does the
clause the annotation is filed under caption that very table?** It separates Issue #700's two
annotations from all nine of the others with nothing in between, and it *ranks* rather than filters
because ADR 0712's placement rule says the outline is one clause out often enough that a filter
would recreate the blindness.

## The blindness underneath, found while looking for the ground

The ground a table stands on could not be counted at all, because `read_tables` took the digits
after `Table ` and stopped. `Scan::designations` is the wider population now — a token carrying a
digit, optionally behind the `Annex ` the caption prints — and `caption_of` reads the same shape off
`doc/md/`'s caption lines, so both sides of every comparison are one rule asked twice. The gate's own
numbered population is untouched and its test says so.

**And a gate for it is named rather than built**: nothing checks a non-numeric designation for
correctness, and the one this tree cites that ISO 32000-2 does not caption is `Table A.19`, which
both places attribute to ISO/IEC 15444-1. A gate would need the foreign-standard rule
`read_citations` already has for a SECTION SIGN, which is a round of its own.

## Gates and sweeps

The full §2 sequence, as this is a fifth round, and §5's binaries with it. §4's sweeps before and
after, against a pristine checkout at the base commit with its own build directory, closed with it.
`doc/errata-read.md` is round 819's file this batch; the edit here is the one paragraph this round
earned, at the end of the sixth blindness's section, so that the merge round can reconcile it
against 819's work.
