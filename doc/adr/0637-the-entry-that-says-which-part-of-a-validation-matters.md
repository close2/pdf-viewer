# 0637 — The entry that says which part of a validation matters

Status: accepted.
Context: the successor selection rule's second use, on §12.8.1.

## The rule, run

ADR 0627 replaced ADR 0567's pairwise ranking with one sentence:

> Rank each live ledger row by the errata annotations that fall on it whose issue number this tree
> names nowhere. Reassemble the issue from every clause `emit` files it under, and read the issue
> whole.

Run at this base it puts §12.8.1 at the head: nine annotations under five issue numbers — #54, #55,
#117, #121 and #219 — with one of the five split across two clause headings. `doc/errata-read.md`
records all five with the rectangle that places each. Three of them are editorial or informative
(a typo's `0`, PDF version markers on `/SubFilter` values, an ordering sentence extended to
document timestamps). Two are not, and neither could have been printed by a gate.

## Decision 1: `/V` is read, and said

**Table 255's `/V` has one sentence addressed to whoever validates**, and until this round nothing
in this tree read the entry:

> The value is 1 if the Reference dictionary shall be considered critical to the validation of the
> signature.

This program evaluates no transform method. §12.8.2.2.2's comparison of two revisions is what that
would take, and the ledger has recorded it as not done since the row was written — so a file that
writes `/V 1` is naming the part of its own validation that this program skips, and the closing
paragraph `viewer_core::notes` prints for every signed document ("of the three questions a signature
asks, this program answers two") is weaker on that file than it sounds.

So `Signature::format_version` reads the entry as the file's own integer,
`Signature::reference_is_critical` applies the entry's own condition, and `notes` says one sentence
where it holds. **The condition is `/V == 1` and nothing is added to it** (trap 11): Table 255 gives
the entry a default of 0, so an absent entry says nothing, and a value the standard gives no meaning
to is a claim about a format edition nobody here has read.

**It is a sentence rather than a verdict, deliberately.** Reading `/V` does not let this program
evaluate a transform method; what it lets it do is stop being silent about the fact that the file
asked for one. That is the same shape as every other line `notes` prints about a signature — the
program says what it did and what it did not, and never the word *valid*.

**The population is measured, not assumed** (trap 8). `examples/signature_algorithm_census` counts
Table 255's `/V` over every document this tree can reach, and it is the command the row cites rather
than a number written down: **no curated document states the entry at all**, so the test's witness is
hand-built, and the `CC-MAIN-2021-31` crawl holds the files that do. The two that write `/V 1` each
carry a `DocMDP` and a `FieldMDP` signature reference dictionary and a `/SigFieldLock` — precisely
the material the sentence is about, which is what makes the report worth its condition rather than a
curiosity.

Calibrated per trap 13, two ways, both from the same fixture: with the guard forced false the test
fails on the signature that states `/V 1`; with it widened to *the entry is present* the test fails
on the signature that states `/V 0`, which six crawled files write.

## Decision 2: the row's claim about Table 255 is corrected rather than reworded

§12.8.1's note opened **"Table 255 entire"** and then listed thirteen of the table's eighteen
entries. Issue #121 is on the `/Filter` row, so reading it meant reading the table against the row,
and the claim did not survive: `/R`, `/V`, `/Prop_Build`, `/Prop_AuthTime` and `/Prop_AuthType` had
no reader.

Four of the five are declined in the row **with the entries' own words**, because a decline that
names the standard's sentence is checkable and one that says "not needed" is not:

- `/R` withdraws itself — "(PDF 1.5) This entry shall not be used, and the information shall be
  stored in the Prop_Build dictionary".
- `/Prop_Build`'s use "is defined by Adobe PDF Signature Build Dictionary Specification", which this
  project does not hold. Reading it would be asserting a fact about a specification nobody here has
  read, which is the rule ADR 0229 states for exactly this situation.
- `/Prop_AuthTime` and `/Prop_AuthType` are "used in claims of signature repudiation" — a proceeding
  rather than a validation, and one this program takes no part in.

§12.8's row said "Table 255 whole" for the same reason and is corrected with it.

## Decision 3: a one-word strike, and what a deprecation does to a debt

Issue #117 strikes `(Required)` from Table 256's `/DigestMethod` and writes **Optional; deprecated
in PDF 2.0**. §12.8.1's row quoted the retired word as the opening of that entry.

**`spec-errata check` cannot see it**, and this is the third blindness `doc/errata-read.md` lists
meeting the one population that has a gate: the strike is a single word, under the four-word floor
that keeps the tool from firing on coincidences, so a quotation resting on retired text read as a
live one. `emit` before writing is the only instrument, which is the rule that file already states.

**The correction is not only to the words.** An entry that was *required* and unread is a debt; an
entry PDF 2.0 deprecates and makes optional is one a reader meets on older files alone, and the row
says so now. The strike also leaves standing the NOTE below it — "[t]he DigestMethod key was also
corrected to be required as no default value is defined" — which now reports a correction the
erratum has undone. **The cell decides and the NOTE does not**: a NOTE is informative and a table
cell is not. This is the second place `doc/errata-read.md` records the collection disagreeing with
the document it amends, after Table 161's letters (ADR 0601), and it is settled the same way — by
reading, not by applying.

## Decision 4: the rule's own step 2 is repaired

Run as `doc/todo/01` wrote it, the recipe ranked §12.7.5.5 first. Both of that row's top two issues
turned out to be **already recorded**, in `doc/errata-read.md`, in a table row — because that file
writes its issue numbers bare, in a column, and step 2's grep asks for the `Issue #` prefix. The
prefix exists for a real reason (`&#124;` is an escaped pipe and collides), so the repair is a union
rather than a replacement: the prefixed grep over the tree, plus that file's own column with numeric
character references stripped.

**A bare-number grep over the tree is not the repair.** `doc/HAYRO_ISSUES.md` and
`doc/HAYRO_ISSUES_FOR_QUORRA.md` are lists of another project's GitHub issues and they name `#54`,
`#55`, `#680` and `#681` — four of the five errata read this round. A number-only search answers
"recorded" from a document about a different tracker.

With step 2 repaired the head is §12.8.1, which is where ADR 0627's reconstruction across nine bases
said it had been at every one of them. **That is the rule confirming itself rather than a
coincidence**: the shortfall moved rows the tree *had* read up the ranking, and removing it put back
the row nobody had been on.

## What was not done, and why

**§12.8.2.2.2's comparison is not implemented here.** Reading `/V` says the reference dictionary is
critical; performing the transform-method comparison needs two revisions of the document and a
digest over an object graph, and that is a round of its own with its own decision to make about what
a mismatch means for a program that states no verdicts. What this round removes is the silence, not
the debt, and the row still says `partial` for the reason it always did.

**Issue #55's ordering `shall` is recorded and not acted on.** "These shall follow the certification
signature if one is present", extended to the document timestamp bullet, is a rule about how a file
is assembled; this program assembles none. A validator could report a timestamp that precedes a
certification signature, and §12.8.5's row already records that no corpus document carries a
document timestamp at all.
