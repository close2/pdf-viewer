# 0743 — The row ranking goes flat, and a correction deleted an entry the table states

Status: accepted.
Context: the errata selection rule's fifteenth use — the sixth consecutive use whose base count
reproduces the previous use's closing arithmetic, and the first whose row ranking has no head.

## The rule, with a fifth step

ADR 0627's rule with ADR 0637's repair to step 2, ADR 0653's tie-break, ADR 0671's fourth step,
ADR 0691's writing rule, ADR 0712's placement rule and ADR 0732's family guard:

> Rank each ledger row by the errata annotations that fall on it whose issue number this tree
> names nowhere, attributing a heading only to a row in the heading's **own** family. Rank once
> over the live rows and once over **every** row, take the head of the two, and prefer the settled
> row where they tie. Reassemble the issue from every clause `emit` files it under, and read the
> issue whole — and a verdict written under a heading is a claim about a page, not about a clause,
> until the rectangle has been placed.

Of the issue numbers in `doc/ISO_32000-2_sponsored_EC3.pdf` that carry a strike or a caret under
the recipe's own single-issue line parse, **61 were named nowhere at this round's base**, of a
population of 302 — the fourteenth use's closing arithmetic, 71 less its ten verdicts, reproduced
by the greps rather than quoted from the record. The multi-issue parse counts 310 and 63, which
are likewise the fourteenth's figures less the same ten.

**And the ranking has nothing left to say.** Both heads are two annotations:

| ranking | head | tied there | one below |
|---|---|---|---|
| live rows | two | 8 rows | 4 rows at one |
| every row | two | 39 rows | 19 rows at one |

The thirty-nine are twenty `implemented`, eight `out-of-scope`, six `partial`, two `reported`, two
`inapplicable` and one `writer-side`. ADR 0653's tie-break was written to settle a tie of three
and cannot settle one of thirty-nine; reading the plateau out would be reading most of what the
rule has left, which is not a ranking at all.

**So the recipe gains a step 5**: where the row counts go flat, rank the same annotations by
**issue** and take that head, saying which unit it came from. This is not a second instrument. Step
3's own last sentence already says to read one issue whole across every heading it appears under —
the issue is the *reading* unit and always was, and the ranking was simply never asked in it. By
issue the population still has a shape: one issue at six annotations, two at four, two at three,
thirty-three at two, twenty-five at one.

**The decay curve is why this is a repair and not a retirement.** The base population has run 133,
111, 104, 99, 85, 73, 71, 61 over the uses that recorded it — six or seven a use, steady — and a
fifth of the collection's strike-or-caret issues is still unread. What collapsed is the *row*
count's resolution rather than the yield: a head of fifteen ranked something and a head of two tied
thirty-nine ways ranks nothing. Retiring the rule here would have read the instrument's exhaustion
as the population's.

One more thing the same arithmetic says, and it is step 4's argument arriving as a measurement:
**43 of the 61 unread issues touch only a settled row and 12 touch a live one** (one touches both;
the rest fall under headings whose family the ledger has no row for). What this rule has left to
find is almost entirely on rows claiming to owe nothing.

## The head, by issue: #346, and it reaches three rows

Six annotations, five of them bare `Caret`s — `check`'s fourth blindness, which leaves no retired
text for a quotation to land on — and every one puts the same two words, *not inheritable*, into a
standard structure attribute's requirement cell:

| table | entry | cell before | cell after |
|---|---|---|---|
| 382 (§14.8.5.5) | `/ContinuedList` | `(Optional; PDF 2.0)` | `(Optional; not inheritable; PDF 2.0)` |
| 382 (§14.8.5.5) | `/ContinuedFrom` | `(Optional; PDF 2.0)` | `(Optional; not inheritable; PDF 2.0)` |
| 385 (§14.8.5.8) | `/Type` | `(Optional)` | `(Optional; not inheritable; PDF 2.0)` |
| 385 (§14.8.5.8) | `/BBox` | `(Optional)` | `(Optional; not inheritable; PDF 2.0)` |
| 385 (§14.8.5.8) | `/Subtype` | `(Optional; PDF 1.7)` | `(Optional; not inheritable; PDF 2.0)` |

`doc/errata-read.md` has every rectangle against `pdftotext -bbox`. The outline filed the first two
under §14.8.5.6 and the last under §14.8.6.2, both one clause late, which is ADR 0712's placement
rule doing this round's work before a verdict was written — and, for the last one, the same
conversion that caused the finding below.

## The finding: a row denies an entry its table states, and a correction is what put the denial there

§14.8.5.8's note read

> Table 385's `/Type` and `/BBox` on an `Artifact` element, whose four kinds — `Pagination`,
> `Layout`, `Page` and PDF 2.0's `Inline` — are the ones §14.8.2.2.2's Table 363 property list
> names. Read and dismissed for the same reason as the rest of §14.8.5: it describes rather than
> draws. (Two corrections in the three-hundred-and-eighty-seventh session: the number said 384,
> which is the table attributes, and the entry list said `/Subtype`, which is Table 363's and not
> this one's — an attribute object here states only the two.)

Three claims, and each is false:

- **Table 385 states three entries.** Its `/Subtype` row is on page 809 of the standard and at line
  17766 of `doc/md/`, printed under §14.8.6.2's heading rather than its own because the table
  straddles a page break. The session that deleted it from the list was reading the same conversion
  that made the ranking offer role maps and namespaces as an artifact attribute's erratum, four
  hundred sessions apart — and **Issue #346's only strikeout is over the requirement cell of the
  entry that sentence deletes.**
- **The two tables are not copies.** Table 363's fourth type name is `Background`, Table 385's is
  `Inline` (PDF 2.0), and their `/Subtype` cells differ with them: Table 385's should appear "when
  the Type entry has a value of Pagination or Inline", Table 363's names `Pagination` alone.
  `structure::ArtifactKind` is Table 363's four and is right for the property list it reads;
  nothing reads Table 385's.
- **`/BBox` is not dismissed; it is read, and was before the erratum.** `Tree::attribute` applies
  §14.8.5.3's priority over every PDF-native owner, so an attribute object whose `/O` is `Artifact`
  answers `Tree::bounds` exactly as Table 379's `Layout`-owned one does. The row said the table was
  read and dismissed while one of its three entries had a reader — the eighteenth sweep's shape
  (`--bin overstated`, ADR 0475) with both of its sides inside one note, which is why no sweep could
  print it.

**And the status stood on a reason its own neighbour had already retired.** "It describes rather
than draws" is word for word what §14.8.5.7's row records being taken off `inapplicable` for, and
what §14.8.5's parent row states the replacement for: neither is drawn and both are read aloud.
§14.8.5.8 is `partial` now — one entry read, two not, with what the two would buy written in the
row: exactly what `Artifact::read` already gives a consumer for the marked-content form of the same
information.

## The second row the same erratum opened

§14.8.5.5 was `inapplicable` on a note naming **one** of Table 382's three entries. `/ListNumbering`
is the one it named and its argument stands — the labels are `Lbl` elements holding marks the page
already shows, so the entry names a scheme for a processor that intends to renumber. PDF 2.0's
`/ContinuedList` and `/ContinuedFrom` are the two the erratum lands on, and the clause says who they
are addressed to:

> The ContinuedList and the ContinuedFrom attributes described in "Table 382 - Standard list
> attributes" control the interpretation of the L element as it relates to other L elements that
> are not its immediate parent.

This program interprets an `L` element — `StandardType::List` reaches a screen reader as a list — so
a flag saying whether "the list is a continuation of a previous list in the structure tree", and an
`/ID` naming which list, have a meaning here. Nothing reads either and nothing reports it, so the
row is **`silent`**: a listener is told a fresh list where the document stated a continuation. That
is debt created rather than paid, which is what the coverage question is for.

## What changes in the code

One doc comment and one test. `Tree::bounds`'s comment named Table 379 alone for a function that
answers from either table's cell, and rested its choice of `Tree::attribute` over
`Tree::inherited_attribute` on the one cell that said *not inheritable*; after #346 both do, and the
comment says so.

`an_artifact_owned_bounding_box_is_the_same_rectangle_as_a_layout_owned_one` pins the reader the row
had denied. Its third element is the discrimination: a `/BBox` under `HTML-4.01`, one of Table 376's
format-specific owners, whose value applies only "if processing based on the format indicated by the
owner value" — so a test of the first two elements alone would pass with the owner ignored
altogether.

## Calibration

Trap 13, above the commit that makes the change, both directions:

| planted | fails |
|---|---|
| `Artifact` dropped from `Owner::is_pdf_native` | first assertion, `left: None` |
| the third element given the `Artifact` owner it must not have | third assertion, `left: Some([12.0, 24.0, 62.0, 84.0])` |

## One question recorded rather than paid

§14.8.5.3's priority 1 names the `NSO` owner outright — "owned by an owner as specified by the O
entry, or, if the value of the O entry is NSO , the NS entry, excluding Layout, PrintField, Table ,
List and Artifact" — so an `NSO`-owned attribute object is never priority 2's, whose five owner
names it cannot carry, and priority 1's condition is one this program does not meet.
`Tree::attribute` admits `Owner::Namespace` beside the five anyway, with no argument anywhere in the
tree. It is a question rather than a defect because §14.8.6.1 makes a standard structure namespace
this standard's own vocabulary rather than an export format's, so the condition may well hold for
some of them; and settling it decides *rank* as well as membership, since priority 1 sits above
priority 2 wherever its condition holds. Recorded in §14.8.5.3's row with `Namespace::is_standard`
named as the predicate either answer needs.

## Consequences

- One erratum gains a verdict, taking the rule's population to 60 — the smallest count any use has
  taken, and worth five table cells across two clauses.
- §14.8.5.5 moves `inapplicable` → `silent`, §14.8.5.8 `inapplicable` → `partial`, and §14.8.5's
  parent row loses a count of its own family that had gone stale by four.
- **`silent` is a status the ledger was not carrying at all**, and that is a consequence worth
  stating rather than a side effect. `CLAUDE.md` calls it "the status worth hunting" and the
  reason a ledger with none of it is not a ledger with nothing hidden: what ships is the gap
  inside a feature that is already there, and the two entries above were sitting inside one under
  a status that says nothing is owed. A row nobody can find is worse than a row that reads badly.
- `doc/todo/01`'s recipe carries step 5, and its own decay reading: the rule is not finished, its
  row unit is.
