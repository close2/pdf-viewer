# 0728 — The rule the writer is named by, and the caret no check can see

Status: accepted.
Context: the errata selection rule's twelfth use — the first whose two rankings top out at the
same height with the live head *inside* the settled plateau, the third consecutive use whose base
count reproduces the previous use's closing arithmetic, and the first whose finding arrives by an
erratum that strikes nothing at all.

## The rule, unchanged

ADR 0627's rule with ADR 0637's repair to step 2, ADR 0653's tie-break, ADR 0671's fourth step,
ADR 0691's writing rule and ADR 0712's placement rule:

> Rank each ledger row by the errata annotations that fall on it whose issue number this tree
> names nowhere. Rank once over the live rows and once over **every** row, take the head of the
> two, and prefer the settled row where they tie. Reassemble the issue from every clause `emit`
> files it under, and read the issue whole — and a verdict written under a heading is a claim
> about a page, not about a clause, until the rectangle has been placed.

Of the issue numbers in `doc/ISO_32000-2_sponsored_EC3.pdf` that carry a strike or a caret under
the recipe's own single-issue line parse, **85 were named nowhere at this round's base**, of a
population of 302 — the eleventh use's closing arithmetic, 99 less its fourteen verdicts,
reproduced by the greps rather than quoted from the record, for the third consecutive use. A parse
that also reads the multi-issue annotation lines counts 310 and 87, which are likewise the
eleventh's figures less the same fourteen.

## The heads: one height, eight rows, and the live head among them

Over **every** row eight rows tie at four annotations — §7.6.4.4.3, §7.10.5.3, §12.5.6.1,
§12.5.6.2, §12.10.4, §12.11.2, §14.7.5.4 and §14.8.5.4.2 — and over live rows exactly one of the
eight is live, §12.5.6.2, `partial`. So the two rankings tie and step 4 prefers the settled row,
which leaves seven to choose between. The third use's tie-break decides: **read the row whose
errata strike a cell ahead of the row whose errata substitute a word in prose.** Six of the seven
move typography, a URL, a step letter, a version marker or a requirement word inside clause 13's
ground; **§14.8.5.4.2**'s two issues rewrite table cells outright, and it takes the head.

**The settled head confirmed its rows and paid nothing, for the fourth consecutive use.** Issue
#223 replaces two of Table 377's *Structure Elements* cells — *Vertical text* becomes a definition
in terms of inline-progression direction, *Ruby text* becomes the three element types by name —
and Issue #189 inserts a cross-reference to §14.8.3.3 into `/Placement`'s and `/WritingMode`'s
descriptions in Table 378. Every attribute concerned is unread here and both rows stay
`inapplicable`. **Placing the rectangles moved the row**: both of #223's pairs are Table 377's, so
the annotations belong to §14.8.5.4.1 and the ranking had named the clause below it.

## The payment: §12.5.6.2, and an erratum that strikes nothing

**Issue #90 is a single caret and there is no strikeout beside it.** It inserts *providing the
Contents key, if* after the opening *When* of §12.5.6.2's paragraph rule, so the amended sentence
reads: when providing the Contents key, if separating text into paragraphs, a CARRIAGE RETURN
(0Dh) shall be used and not, for example, a LINE FEED character (0Ah).

The rule as published is passive, and a reader that enforced it would show a producer's paragraph
break as a space. This tree therefore accepts a line feed as a paragraph break as well — in
`viewer_ui::chrome`'s popup window and in `variable_text::encode`'s free text layout, which is the
`/Contents` a free text annotation draws on the page — and argued that in both places as an
*inference* about whom the `shall` binds. The amended sentence opens by naming the act of writing
the entry. The behaviour does not move; its warrant stops being ours and becomes the clause's, and
the entry the amendment scopes to is exactly the one both readers lay out.

**Issue #297 re-dates the same row's other quoted sentence.** Grouping by `/IRT` becomes PDF 1.5's
and *(PDF 1.6)* is inserted after the `RT` instead, which is what Table 172's own `/RT` row has
always said. Two struck words — under the four-word floor `spec-errata check` filters on — and the
sentence is a rustdoc blockquote in `markup.rs`, the same blockquote in
`examples/annotation_group_census.rs`, and a quotation in the row. All three keep the published
wording the quotation gate verifies against `doc/md/` and carry the amendment beside it; nothing
here gates on a version, so no behaviour moves.

## The walk downward, in one line each

- **§7.6.4.4.2** (`partial`, and filed under §7.6.4.4.3 by the straddle) — Issue #643 strikes the
  `b` of Algorithm 3's two cross-references into Algorithm 2 and writes *a*. Algorithm 2's step
  (b) initialises the MD5 hash function; step (a) is where the padding string is printed, so the
  published references sent an implementer to a step that pads nothing. `crypt.rs` has cited step
  (a) since `PAD` was written; the erratum vindicates the reading.
- **§14.7.5.3** (filed under §14.7.5.4) — Issue #339 strikes the *used* of Table 358's `/Pg`
  sentence for *required*, inside the run Issue #431 replaces wholesale. `destination.rs` credited
  #431 with both halves and now names each: **two accepted errata over one run, agreeing** — Table
  161's shape with the contradiction removed.
- **§14.7.5.4** — Issue #343 strikes the first *at least one* of the parent tree's own sentence
  for *a*, leaving the second standing. Three words, under `check`'s floor, quoted in
  `content/marked.rs` and in the row; what makes the route per stream is the sentence's second
  half and Table 359.
- **§12.10.3** (filed under §12.10.4) — Issue #321 takes the *both* out of "shall be described in
  either or both of two well-established standards", where each of Table 270's two rows forbids
  the other entry: **the published clause contradicted its own table.** `CoordinateSystem::is_stated`
  reads the table.
- **§12.11.2** — Issue #195 demotes *shall apply* to *applies* in Table 275's `U3D` and `PRC`
  cells, inside `CLAUDE.md`'s multimedia exclusion.
- **§7.10.5.2** (filed under §7.10.5.3) — Issues #269 and #669 correct the capital *If* of Table
  42's conditional-operators cell and the hexadecimal *07hD* of RIGHT CURLY BRACE.
- **§12.5.5** (filed under §12.5.6.1) — Issue #422 italicises two leading `f`s in an EXAMPLE.

No code moves this round: every read either confirmed a row or corrected a quotation, and the two
that could have moved behaviour were already implemented the amended way. `doc/errata-read.md`
carries all twelve with their rectangles.

## What this adds to the rule's record

- **`check`'s fourth blind spot is a caret with no strikeout.** The three recorded before it are
  an addition over text nobody has quoted, a strike under the four-word floor, and a spelling
  `doc/md/` writes differently — all of them about *what was struck*. This one is about an
  insertion into a sentence this tree quotes **correctly**: there is no retired text, so no
  quotation can land on it and no direction of `check` can report it. `emit` read against the
  rectangle is the only instrument, which is the fourth reason for the rule that a round
  implementing a clause runs `emit` before it writes.
- **The errata that pay are not the ones that change the most words**, and the ranking cannot see
  that. Of the twelve read here the two that moved something in this tree are a caret of five
  words and a strike of two, while the four that rewrote whole cells confirmed their rows. The
  ranking's unit is the annotation; what does the work is step 4's practice — the head to a
  verdict, then downward until a row pays.
- **The page-straddle was the majority case.** Seven of twelve issues are filed one clause late,
  and twice that moved the row a verdict belongs to: the settled head's annotations are the row
  *above* the one it named, and an `implemented` §7.6.4.4.3's four belong to its `partial`
  neighbour — the ranking crediting a settled row with a live row's errata. ADR 0712's placement
  rule did most of this round's work before a verdict was written.
- **The base count agreed with the derivation for the third consecutive use.** The greps remain
  the instrument; the record remains a derivation.
