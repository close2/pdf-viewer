# 0732 — The array that states no children, and the guard written for exactly that contradiction

Status: accepted.
Context: the errata selection rule's thirteenth use — the fourth consecutive use whose base count
reproduces the previous use's closing arithmetic, the first whose settled head *pays code*, and
the first to find a defect in the recipe's own step 3.

## The rule, unchanged

ADR 0627's rule with ADR 0637's repair to step 2, ADR 0653's tie-break, ADR 0671's fourth step,
ADR 0691's writing rule and ADR 0712's placement rule:

> Rank each ledger row by the errata annotations that fall on it whose issue number this tree
> names nowhere. Rank once over the live rows and once over **every** row, take the head of the
> two, and prefer the settled row where they tie. Reassemble the issue from every clause `emit`
> files it under, and read the issue whole — and a verdict written under a heading is a claim
> about a page, not about a clause, until the rectangle has been placed.

Of the issue numbers in `doc/ISO_32000-2_sponsored_EC3.pdf` that carry a strike or a caret under
the recipe's own single-issue line parse, **73 were named nowhere at this round's base**, of a
population of 302 — the twelfth use's closing arithmetic, 85 less its twelve verdicts, reproduced
by the greps rather than quoted from the record, for the fourth consecutive use. The multi-issue
parse counts 310 and 75, which are likewise the twelfth's figures less the same twelve.

## A defect in step 3, found before the head was

Step 3 attributes each `emit` heading to *the nearest ledger row at or above that clause number*,
and the ledger carries rows for the technical clauses and for the **normative** annexes alone —
D, E, F, I, K, L, O and Q, which is `CLAUDE.md`'s list. Annexes A, B, C, G, H, J, M, N and P say
*informative* on their own title lines and have no rows at all, so every annotation under one of
them attributes to whichever row sorts below it, and under the ordering this round's arithmetic
used that is **the last technical row in the ledger**.

Run that way the head of the full ranking is §14.13.10 with six annotations, and four of the six
are Annex A's; Annex H's two land on Annex F's last row, which is the nearest row above `H` in the
same ordering. The repair is a *family guard* — attribute a heading only to a row whose top-level
clause number or annex letter is the heading's own, and count what has no row in its own family
separately. Twelve annotations are in that state: six under the front matter, whose clauses 2 and
3 the ledger starts after and which the unguarded arithmetic already dropped in silence, four under
Annex A and two under Annex H. With the guard, §14.13.10 keeps its own two and the real heads
appear.

**It is a decay detector's own decay.** The ranking gets sharper every use because reading takes
issues out of the population, so a noise source that was buried under real heads floats to the top
exactly when the real heads get short. Six of noise beat three of signal on the first use where
the signal was three.

## The heads: a tie at three, and the settled row wins it

With the guard in place, over live rows the head is **§12.7.4.1** with three annotations,
`partial`; over **every** row seven rows tie at three, one of them §12.7.4.1 itself. Step 4 prefers
the settled row where the two tie, and the third use's tie-break decides between the six: five move
a word in prose, an EXAMPLE's numbering, a spelling, a linearisation version or clause 13's ground,
while **§7.7.3.2**'s three carets put three requirements into Table 30's cells. It takes the head,
`implemented`.

**One of the three is not §7.7.3.2's.** Issue #614's caret sits on page 117 at
`[285.9199 481.29234 294.99595 488.68763]` — (353.2)–(360.6) from the top of an 841.92-point page
— where `pdftotext -bbox` puts the `2.0)` that closes `(Optional; PDF 2.0)` at x 277.89–294.37 on
Table **29**'s `/DPartRoot` row. §7.7.3.2's heading is at (732.0) on the same page, so the outline
files the catalogue's own table under the page tree node's clause: ADR 0712's rule again, applied
before the verdict. The caret writes *; shall be an indirect reference*, which is the third member
of the family §7.7.2's row already records for `/Extensions` and `/StructTreeRoot` and takes the
same reader's tolerance — `Document::get_key` resolves either shape.

## The payment: Issue #271, three insertions, and the one that cost a page

Two carets on page 118, `Review/Completed`, both of them **carets with no strikeout** — the fourth
blind spot ADR 0728 named, met again the round after it was named.

- `[493.48 610.979 501.433 617.459]` — (224.5)–(230.9) from the top, its centre at x 497.5 where
  `-bbox` ends the `nodes.` of *node. The children shall only be page objects or other page tree
  nodes.* at 499.33 — writing *(null entries shall not be present). The length of the array shall
  be at least one*, into Table 30's `/Kids` cell.
- `[323.298 574.979 331.251 581.459]` — (260.5)–(266.9), its centre at 327.3 where the `tree.` of
  *this node within the page tree.* ends at 329.47 — writing *which shall be 1 or greater*, into
  the `/Count` cell.

**Two of the three vindicate the code.** `Node::of` answers `None` for a `/Kids` entry that is a
null, so such an entry is stepped over without consuming one of the pages the walk is counting
down; the erratum turns that from this reader's reading of *the children shall only be page
objects or other page tree nodes* into the cell's own words. And both places that trust `/Count`
require it positive — `Pages::new` before believing a declared page count, `find_leaf` before
skipping a subtree — which was written as a plausibility test and is now the entry's own floor.

**The empty array is what moved, and the guard it slipped past was written for it.** `Pages::new`
declines to believe a `/Count` on a root with no `/Kids`, on the argument its own comment gives:
Table 30 requires both entries, so a root stating a count and no children has contradicted itself
and the walk settles it. The test for *no children* was `as_array().is_some()`, which an empty
array passes. So:

- a root writing `/Kids [] /Count 3` kept its `/Count` authoritative: `Pages::len` answered three,
  every `Pages::get` answered `None`, the recovery scan never ran because the count was not zero,
  and the file's own page object sat there unreachable. Three pages claimed, none produced, in
  silence.
- a *child* node writing an empty `/Kids` beside a positive `/Count` was skipped by `find_leaf` as
  a subtree of that many pages, so the skip consumed pages the node does not have and **every page
  after it answered to an earlier page's number**.

Both places ask for a non-empty array now. Nothing else changes: a node with an empty `/Kids` is
still a node rather than a leaf — reading it as a page would draw a node, which is ADR 0305's whole
subject — and `count_leaves` already summed it to nothing.

## What the population says, and why the corpus is quiet

`examples/kidless_node_census` counted the absent entry and not the empty one; it counts both now,
which is one predicate beside the other in the sweep that already walks the tree with `pdf_syntax`
rather than through the code under test (trap 8). Over `doc/pdf.js` and `doc/corpora`, 1231
documents open and **1 states an empty `/Kids`**: `doc/pdf.js/test/pdfs/issue8088.pdf`, whose empty
node writes `/Count 0` beside it — the value Issue #271's third insertion outlaws in the same
breath as the empty array.

That zero is why the document reads correctly under either version of the code: the skip is taken
only on a positive count. It is pinned as a third witness in `page_tree_nodes.rs`, and it is the
end of the pair that says this change is not an over-correction on a real file — the fixtures write
the counts the erratum forbids one way, the witness writes the one it forbids the other way, and
all three keep their pages.

Calibrated per trap 13, both ways, above a commit: with the empty array read as children again both
new tests fail and the seven older ones pass; with `/Count` never believed at all the new tests pass
and the pre-existing `a_count_without_kids_is_not_believed` fails on *a stated /Count over a real
tree is believed*. The fix is bracketed by tests on both sides.

## What this adds to the rule's record

- **A settled head paid code for the first time.** Four consecutive uses had a settled head that
  confirmed its row and paid nothing, and ADR 0708's practice — head to a verdict, then downward
  until a row pays — was what did the work each time. Here the head itself paid, so the walk
  stopped at it; the live head it tied with, §12.7.4.1 at three annotations, is left in the
  population deliberately and is the next use's first candidate.
- **The settled-row mechanism appears for a sixth time and by a sixth shape.** A round trip that
  could not fail (ADR 0671), a sentence about a sibling row's status (0681), a set with no closure
  check (0691), a claim of two written forms with a test of one (0708), a rule satisfied by
  construction whose fixture was too small (0712) — and now a **guard whose predicate was one
  degree weaker than the contradiction it was written for**. `implemented` was true of the shape
  the row's own comment described and false of the shape one word away from it.
- **The recipe's arithmetic is a claim too, and it decays like a row's.** Step 3's attribution rule
  was written for the technical clauses and is silent about an annex the ledger has no row for.
  Nothing about it was wrong when it was written; what changed is that reading the population down
  made the noise the head. The step now carries the family guard and the separate count.
- **The base count agreed with the derivation for the fourth consecutive use.** The greps remain
  the instrument; the record remains a derivation.
