# ADR 0342 — The accessibility ratchet, and the page a map answered for twice

Status: accepted, 2026-08-14. Session 507. Builds ADR 0323's third instrument and closes
`doc/todo/05`'s build list. Amends §14.7's and §14.7.5.2's ledger rows. Fixes a defect in
`viewer-core` that the instrument found on its first run; changes nothing ADRs 0214, 0301, 0325 or
0338 decided.

## The instrument, and why it is a ratchet

ADR 0323 designed three judges for the interactive surface and gave each a different verdict
shape, because the three capabilities have three different kinds of comparable answer. The first
two are built (ADRs 0333, 0334). The third is the accessibility tree, and its design is the one
that has to say out loud what it cannot do:

> **No reference implementation puts a comparable tree on AT-SPI, so there is no oracle here and
> this design does not pretend one.** A count that cannot fall is weaker than a judge that
> disagrees; stating that plainly is the design.

So this is a census with a ratchet's shape rather than a gate with a verdict's, and what makes its
counts worth printing is that each one names a *defect class*. `crates/viewer-core/tests/accessibility_census.rs`
is the instrument and `tools/state.sh accessibility` is how it is asked, the section ADR 0323 said
it should have "from the day it exists, printed not stored".

### The first count decides the shape of the rest

**Pages that answer at all.** Until ADR 0325 every page but the first hundred of a large tagged
document answered `Query::AccessibilityTree` with an empty list — and an empty list is exactly what
an *untagged* page answers, so a screen reader was told a thousand-page tagged document says
nothing about itself, with no report, no count and no gate able to see it. That is what this count
is for, and it is why an empty answer is **classified** rather than merely counted. §14.7.5.4's
structural parent tree is what classifies it, in three cases: the file names elements for the page
and the answer is empty (the defect class); the page states no `/StructParents`, so the walk falls
back to the whole document's tree (ADR 0325's first recorded residue); the file names no elements
for the page (honest).

**The predicate is not independent of the answer, and the instrument says so rather than hiding
it** (trap 8: a measurement taken with the instrument under test is not independent of it). Both
sides read the same file with the same crate. What makes the comparison worth making is that they
read *two different statements the document makes about itself* — §14.7.5.4's parent tree keyed by
the page's own `/StructParents` on one side, and §14.7.2's `/K` walked down from the root on the
other. A file whose two chains disagree is exactly what ADR 0325 was about, and a census that
asked one chain twice would have seen nothing. It is weaker than an oracle and it is what this
surface admits.

### The rest of the counts, and the one assertion

Documents with structure by two predicates the standard keeps apart — a `/StructTreeRoot`, and
§14.8.1's "[a] tagged PDF document shall contain a mark information dictionary … with a value of
true for the Marked entry" — printed side by side with the documents that state one and not the
other. Then what the tree carries: elements reached, §14.9.3's `/Alt` or §14.9.5's `/E`, elements
placed by Table 379's `/BBox` or §12.5.2's `/Rect`, §14.8.4.8.3's resolved header cells, and
§12.7.5's controls behind §14.7.5.3's object references — the censuses `doc/todo/31` already named
as examples, promoted to printed counts.

**ADR 0323 asked for poppler's `/MarkInfo` count beside ours and this computes it from the file
instead.** The predicate is the document's, not poppler's: `pdfinfo`'s `Tagged:` line *is*
`/MarkInfo` `/Marked`, so what a subprocess per document would add is a second reading of an entry
this tree already reads, at the price of making a ratchet depend on a reference. The reference
reading was taken by hand this round instead — `pdfinfo` over the pdf.js corpus answers 78, which is
what ADR 0323 measured and what this census's own predicate answers over the same population — and
the command stays one line away for anybody who wants it again.

**Two things are asserted from the first run, and both are decisions rather than counts**: no input
panics (principle 1), and no untagged page is answered with structure it does not state, which is
ADR 0214's decision that reading order is what §14.7 exists to state and a guess presented where a
person expects the author's answer is worse than the honest silence. The counts themselves are
printed and **not** ratcheted, which is ADR 0323's own rule: an instrument's numbers enter
`doc/todo/02` §2 only after they have held across rounds.

### The denominator, and why `doc/` is in it

Page one of every document in `doc/pdf.js/test/pdfs` and in `doc/`, and **every** page of every
document that states a structure tree. The specifications in `doc/` are in the population
deliberately and it is not a convenience: the pdf.js corpus's tagged documents are 17 pages at
their largest, and the defect the first count is about needs a document big enough for a bound to
run out. This is the corpus-goes-quiet failure one axis over — a population that cannot exhibit the
defect would have made the instrument report success on its first run.

## What the first run found

**Ten pages that should answer and did not, and every one of them page one of a tagged
document** — including ISO 14289-1, which is PDF/UA itself, and ISO/TS 32005, Well-Tagged PDF,
the Tagged PDF Best Practice Guide and four PDF Association application notes. Each states a
`/StructParents`, each has 10 to 26 elements named for it by §14.7.5.4, and each answered a screen
reader with nothing at all.

The cause is one line, and it is a map being read backwards.

`pdf_model::Pages::indices` answers *object → index*, and it deliberately holds an entry for an
intermediate `/Pages` node as well as for each page, because a destination may name a node and the
first page beneath it is the only page such a reference could mean. Its own documentation says so.
`viewer-core` needed the other direction — Table 355's `/Pg` names a page **object** while this
crate holds an index — and got it by scanning the map for the first entry whose value is the index
wanted. A `BTreeMap` iterates in ascending object number, so where an intermediate node's object
number is lower than its first page's, the scan answers with **a node that is not a page**. Every
`/Pg` comparison then failed, every element was pruned as belonging elsewhere, and §14.7's whole
answer for that page was the silence an untagged page gives.

Three call sites had it: `accessibility`, `logical_selection` (§14.8.2.5's order for a copy) and
`move_focus` (§12.5.1's tab order). All three now ask `pdf_model::Page::id` — "which object the
page *is*" — through one helper, `page_object`, whose doc comment carries the trap. One descent to
the leaf is also cheaper than the whole-tree walk the map costs, which is a side effect rather than
the reason.

**What the fix moved**: pages answering 1487 → 1501, the defect class 10 → 0, elements reached
102 572 → 102 849, `/Alt` carried 635 → 664. The counts of the first *sound* run are in session
507's history file, where a number belongs.

**And the lesson is not about page trees.** A lookup table with a deliberate many-to-one entry has
no inverse, and the failure is silent in exactly the direction that matters: the map answered, the
answer was well formed, and it named the wrong kind of object. `doc/habits.md` carries it beside
the other things a data structure claims.

## What the run says about `doc/todo/31`'s two residues

Both were recorded without a number beside either, and now they have one:

- **A page of a large document that states no `/StructParents` still falls back to the whole-tree
  walk.** 56 pages take that fallback and answer empty, and the census prints the size of each
  document's tree beside the page: the largest is 393 elements against the walk's bound of 8192, so
  every one of the 56 is the *file* naming nothing on that page rather than the bound running out.
  `comments.pdf` is the shape of it — a whole tree of one `Figure` whose only content item is an
  annotation on page six, so its other thirteen pages have nothing to say. **The residue has no
  witness in this population**, which is a fact about the population as much as about the code: the
  census names one the day a document exhibits it.
- **An answer cut at the bound.** No page's answer reaches 8192 nodes, over every page of every
  tagged document this project holds. The bound is real and nothing in `Answer::Accessibility` could
  say it had been hit — that shortfall stands and stays on `doc/todo/31` — but it binds nothing
  today, and the count is what will say when it starts to.

## Consequences

- ADR 0323's three instruments are built, and `doc/todo/05`'s build list is closed; what the file
  keeps is the standing rule that a number enters §2 only after it has held.
- `tools/state.sh accessibility` prints the counts; no document holds them, which is `CLAUDE.md`'s
  rule and the reason the ratchet can be trusted next round.
- A defect that made this program lie to a screen reader about page one of PDF/UA's own standard is
  fixed, and it was found on the instrument's first run — which is the argument for instruments
  that ADR 0323 made in the abstract, now made in the particular.
- Two of `doc/todo/31`'s open entries are now measured rather than suspected, and neither has a
  witness today.
