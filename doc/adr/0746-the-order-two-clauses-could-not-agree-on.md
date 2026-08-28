# 0746 — The order two clauses could not agree on, and the tables no instrument watches

Status: accepted.
Context: the errata selection rule's sixteenth use — the seventh consecutive use whose base count
reproduces the previous use's closing arithmetic, and the second run under step 5.

**0745 is a sibling round's.** This number was taken two above the tip on that reservation.

## The rule, unchanged, and what running it a second time under step 5 says

ADR 0627's rule with ADR 0637's repair to step 2, ADR 0653's tie-break, ADR 0671's fourth step, ADR
0691's writing rule, ADR 0712's placement rule, ADR 0732's family guard and ADR 0743's fifth step.
Nothing in it is amended here; one thing it never stated is now written down, and one thing about
step 5 is now known that a single outing could not have told anybody.

**The row ranking is flat and getting flatter.** Both rankings top out at two annotations: over the
live rows seven tie there, over every row 36, with 19 more at one. The fifteenth use found eight and
39. A plateau that *shrinks* as verdicts land — rather than resolving into a head — is the row
unit's exhaustion showing itself a second time, and it is the reason step 5 exists.

By issue the population still has a shape: two issues at four annotations, two at three, 28 at two,
21 at one. Of the 302 issue numbers carrying a strike or a caret under the recipe's own single-issue
line parse, **60 were named nowhere at this round's base** — the fifteenth use's 61 less its one
verdict, reproduced by the greps rather than quoted from the record. The multi-issue parse counts
310 and 62, which are likewise the fifteenth's figures less the same one.

### What the two parses are, which seven uses have re-derived and none had defined

An `emit` line's subject is Table 172's `/Subj`, and a few of them name two issues rather than one.
The **single-issue line parse** keeps the lines carrying exactly one issue number; the **multi-issue
parse** takes every number on every line. Eight numbers appear *only* as the second number of a
two-issue line, and that is exactly the gap between 302 and 310. The definition is now in the recipe
in `doc/todo/01`, because a figure seven consecutive uses have reproduced deserves to be reproducible
without rediscovering what it counts.

**And the field is `/Subj` rather than `/T`, which is now established rather than assumed.** Every
one of the collection's 957 `StrikeOut` and `Caret` annotations carries both; in **two** of them the
two disagree, both on page 715, both belonging to this round's own head — `/Subj` names one issue and
`/T` names its predecessor. Keyed on `/T` that head would hold two annotations instead of four and
would not be a head. How the disagreement was settled is below.

## The head: a tie of two, settled by the row ranking's own tie-break

Issue #163 and Issue #700 each carry four annotations, three times the median and twice the next
tier. The third use's tie-break — a cell, a requirement level, a type or a description ahead of a
word in prose — takes Issue #163: one of its carets writes into Table 333's own cell and the other
three turn a description into a `shall`, where Issue #700's substitute a table's number.

**The head confirmed and paid nothing**, because §13.7.2.1 and §13.7.2.2.1 are `out-of-scope` under
`CLAUDE.md`'s clause-13 exclusion. A `shall` inside an excluded clause is still excluded; an
exclusion is revisited by argument, never by an erratum landing in it. That is the cheapest verdict
this rule produces and it took minutes, which is the outcome ADR 0736 already recorded once.

**The walk downward paid three times**, which is step 4's practice — head to a verdict, then
downward until a row pays — arriving in the issue unit.

## The finding: a documented choice whose warrant an erratum removed

`Articles::page_array_agrees` compared the **set** of beads a page's `/B` array names against the set
the threads put on that page, and said so twice — in the module comment and in its own — on this
warrant:

> The two clauses do **not** agree about the order, and this is the standard contradicting itself
> rather than a file doing it. […] Comparing the set rather than the sequence is a **documented
> choice**, and it is the only comparison both sentences license.

Table 31: "[t]he beads shall be listed in the array in natural reading order." §12.4.3: "the page
object … shall contain a B entry whose value is an array of indirect references to the beads on the
page, in drawing order."

**Issue #320 strikes one word from each.** *natural* goes from the first with nothing in its place;
*drawing* is struck from the second and a caret writes *reading*. Both amended sentences ask for
reading order, and the warrant above is false.

**This is the case where an erratum is decisive, and the distinction is worth keeping.** ADR 0601
established that an accepted erratum is *evidence* about the standard rather than a corrected
standard, on a pair of accepted errata that could not both be applied — and that where they
disagree, the published clause and its own arithmetic decide. Here it is the **published clauses**
that disagree, irreconcilably: a reader cannot obey both, so the choice has to be made from
somewhere, and the collection is the only evidence there is about which way the standard meant it.
Declining to use it would not be caution; it would be preferring a coin toss to the only witness.

### What changed, and what deliberately did not

The comparison is on the **sequence** where the beads on the page all hang on one thread, since a
chain from `/F` along `/N` is that article's reading order; it stays on the **set** where two or more
threads put beads on one page, because neither sentence — published or amended — says how two
articles sharing a page are ordered against each other. The fallback keeps its cost in writing: on
such a page a `/B` in the wrong order is not reported.

`page_array_agrees` has no consumer outside its own module and no corpus document states a `/B` at
all, so nothing a page shows moves. What moves is a report's discrimination, which is what the entry
is read for at all (Table 31 NOTE 2: `/B` "can be created or recreated from the information obtained
from the Threads key").

## The second finding: a table renumbering that no instrument in this tree can see

Issue #700 renumbers Annex O's two tables — Table Annex O.3 becomes Table Annex O.1, Table Annex O.4
becomes Table Annex O.2 — with a strikeout over each caption's designation and a caret carrying the
replacement.

The standing answer to a number an erratum moves is `doc/errata-read.md`'s and is not changed here:
the published numbers stay, because `doc/md/` is what every citation resolves against and the
conformance gate refuses a number ISO 32000-2 does not have; the row is where the amendment is
recorded; and `spec-errata moved` is what makes it findable from outside the row. **The third part
fails here.** `moved`'s predicate is one of four verbs (*move*, *renumber*, *delete*, *insert*) in the
annotation's own `/Contents` **and** a clause number named there. Issue #700's contents are the
replacement text alone and what it renumbers is a table. `check` is blind for two independent
reasons: the struck text is two words, under the four-word floor, and no quotation lands on a table's
caption anyway.

**75 lines across 27 files in this tree name the retired numbers** at this round's base — three
ledger rows, seven crates, five ADRs, two `doc/todo` files and one session record — which is more
ground than any clause renumbering `moved` has reported. The
three ledger rows now record what the tables become. The rest is recorded as a shape rather than
built, which is what the fifth blindness got in ADR 0736 and for the same reason: the next erratum of
this kind will be invisible in exactly the same way, and naming the predicate is what makes building
it a decision rather than a rediscovery. **The predicate**: a `StrikeOut` whose covered text is a
table designation, paired with a `Caret` on the same page whose contents are another — no verb,
because a bare pair carries none, and no clause number, because a table is not one; and the ground
it stands on is a text search for the caption rather than `conformance::citation`'s section signs.

## The third: two `implemented` rows vindicated, and a clause number that was wrong all along

Issue #376 makes Th *the normalized value of the operand to the Tz operator*, whose default operand
of *100%* is *a scaling value of 1.0 for Th*. §9.3.4's row had already written "Th is a factor rather
than the percentage the operator takes" as its own inference; it is the standard's sentence now. The
same issue instructs that §9.4.4's NOTE 2 — the text rendering matrix Trm — become normative text,
which is how any renderer has to treat it and how this one always has. `emit` files that annotation
under §9.5, one clause on, which is ADR 0712's placement rule for the third consecutive use.

Reading it found something no gate can: `variable_text.rs` cited **§9.3.5** for `Tz`, and §9.3.5 is
leading. `tools/conformance` verifies that a cited clause exists and this one does — the same shape
`--bin tables` exists for one level down, where a table number that exists and names the wrong table
reads exactly like a right one.

## The tracker, weighed and not adopted

The project owner has named `https://github.com/pdf-association/pdf-issues` a secondary reference.
Its README calls it a means of "openly reporting errata against any PDF specification, including ISO
publications", CC-BY-4.0, and says resolutions pass a PDF Association working group and "will be
passed to the appropriate ISO working group for formal ratification" — so a resolution there is
industry consensus not yet ratified by ISO, one step further from the text than the annotated
collection, which *is* those resolutions written onto the published page.

It bears on these instruments in exactly one way and is used for exactly that: **identifying which
erratum an annotation belongs to**, a question about the file rather than about the standard. It
settled the `/Subj`-against-`/T` disagreement above — the issue `/Subj` names is titled after
§13.7.2.1 and the RichMediaSettings uniqueness sentence, the issue `/T` names is about a named
destination's `/SD` entry in §12.3.2.4 — after a calibration on an issue this tree already has a
verdict for, so that the numbering was confirmed to be the same numbering before it decided anything.

It is **not adopted**: it changes no population, settles no reading, is not added to `doc/stack.md`
or `doc/third-party-data.md`, is fetched by nothing and gated on by nothing. A round that took a
clause reading from an issue thread would be doing to the tracker what principle 5 forbids doing to
poppler. `doc/errata-read.md` carries the argument where a round reading errata will meet it.

## Calibration

Trap 13, above the commit that makes the change, both directions:

| planted | fails |
|---|---|
| the sequence compared unconditionally | third assertion, `left: Some(false)` |
| the set compared unconditionally, as before | first assertion, `left: Some(true)` |

## Consequences

- Four errata gain verdicts, taking the rule's population to 56 — the largest single-round fall this
  rule has taken, and the smallest count any use has left behind.
- `Articles::page_array_agrees` compares an order where one is stated, and `article.rs`'s module
  comment no longer says the standard contradicts itself about it without saying what removed the
  contradiction. §12.4.3's and §7.7.3.3's rows carry the amendment.
- §9.3.4's and §9.4.4's rows record what Issue #376 vindicates; `variable_text.rs`'s clause number is
  right.
- The three Annex O rows record what Issue #700 renumbers, and the sixth blindness is on
  `doc/errata-read.md`'s list with the predicate a sweep for it would ask.
- `doc/todo/01`'s recipe states what its two parses count, which seven consecutive uses had
  re-derived without it.
- **Step 5's verdict after two outings**: it discriminates where the row unit no longer does, and it
  does not remove the need for either the tie-break or step 4's downward walk. Its head tied at two
  issues, the tie-break settled it, and the head confirmed while the three below it paid.
