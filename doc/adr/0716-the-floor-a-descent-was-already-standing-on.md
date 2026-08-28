# 0716 — The floor a descent was already standing on

Status: accepted.
Context: the errata selection rule's tenth use — the first time the live head moved by decay
alone, the fifth time the full ranking out-ranked the live one, and the first time a round's
base count reproduced the previous use's closing arithmetic exactly.

## The rule, unchanged

ADR 0627's rule with ADR 0637's repair to step 2, ADR 0653's tie-break, ADR 0671's fourth step,
ADR 0691's writing rule and ADR 0712's placement rule:

> Rank each ledger row by the errata annotations that fall on it whose issue number this tree
> names nowhere. Rank once over the live rows and once over **every** row, take the head of the
> two, and prefer the settled row where they tie. Reassemble the issue from every clause `emit`
> files it under, and read the issue whole — and a verdict written under a heading is a claim
> about a page, not about a clause, until the rectangle has been placed.

Of the issue numbers in `doc/ISO_32000-2_sponsored_EC3.pdf` that carry a strike or a caret
under the recipe's own single-issue line parse, **104 were named nowhere at this round's base**,
of a population of 302 — which is the ninth use's closing arithmetic, 111 less its seven
verdicts, reproduced by the greps rather than quoted from the record. It is the first time the
two figures have agreed since the record started carrying them, and it follows two rounds of
corrections to exactly this count: the eighth use's off-by-one, and the ninth's rule that a
closing figure is a derivation.

## The heads: a settled row over the live one, and the practice that reads both

Over live rows the head is **§9.8.1 with six annotations under three issues** — the pair that
had led since the seventh use left the ranking when the ninth read it, so the live head moved
for the first time in five uses, by decay rather than by anybody's choice. Over **every** row
the head is **Annex L with seven under two issues, `writer-side`**, so step 4 takes the settled
head, and the eighth use's practice follows: the head to a verdict, then downward until a row
pays.

**The settled head confirmed its row and paid nothing, which is the eighth use's outcome
again and is legitimate.** Issue #83 admits `Table` as a child of `P` (0..n, both directions),
stated as an inserted NOTE because the published sections list the pair in neither; Issue #440
corrects the `WP`/`Figure` pair's two cells from `c` — **a value Table L.1's legend never
defined**, so the published normative matrix constrained that pair with a constraint that had
no meaning — to 0..n. Nothing in this tree reads a cell of Table L.2; the row promises the
table to a checker of tagged PDF, and its note now carries the amended cells for whoever
builds one.

## Where it paid: the live head, three times

**Issue #190 rewrites `/Descent`'s floor, and the code's stated choice becomes the entry's own
words.** The published sentence — "[t]he value shall be a negative number" — is amended to
*a number less than or equal to zero*, with an inserted NOTE that font programs write descender
metrics in either sign while PDF always expects negative values. Three things stood on the
struck sentence:

- three rustdoc blockquotes — `pdf_font::metrics::measured_extent`'s, which the conformance
  gate verifies, `pdf_model::variable_text::Metrics::read`'s and a test's — all on a one-word
  strike below `spec-errata check`'s four-word floor, the fourth of this rule's uses to find
  quoted text under it;
- `measured_extent`'s acceptance of a zero descent, argued in its doc comment as this program's
  reading of a depth against a sign convention — now the clause's own permission, which is
  principle 5's recurring shape: the code was right for a reason nobody had found;
- its own test's comment, which credited Table 120 with permitting zero — true of the amended
  table before it was true of the published one.

The inserted NOTE also names the mechanism behind the corpus's 42 positive descents — a
producer copying its font program's convention — corroborating ADR 0216's repair without
legalising the form. Every blockquote keeps the published wording the gate verifies against
`doc/md/`, with the amendment in prose beside it, per the standing convention for an erratum's
added text.

**Issue #152 makes Table 257's `/P` an integer, and closes a misread window no test could
see.** `signature::modification` has always read the entry with `as_integer`; under the
published type cell — `number` — a conforming file could write `/P 1.0` as a real, and
`as_integer` read it as absent: the table's default, level 2, in place of the level 1 the file
wrote. A permission-widening misread of a conforming file, invisible to every fixture because
every fixture wrote the level as the integer it is — the settled-row mechanism's shape on a
live family, a read satisfied by construction with nothing that could fail its alternative.
The amended cell makes that file malformed and the integer read exact.
`a_docmdp_level_written_as_a_real_takes_the_tables_default` pins the recovery — a value the
amended table does not admit restricts no further than an absent one, the same stance as the
row's existing rule for an integer outside 1..=3 — calibrated per trap 13 against the
numeric-read plant, which passes all 36 pre-existing signature tests and fails only the new
one, run both ways and reverted.

**Issue #474 widens `/FontWeight` to an integer between 1 and 1000 inclusive**, the published
nine hundreds becoming a `should` — OpenType's usWeightClass range, which is what a variable
font's instance states. `substitute.rs`'s bold threshold at 600 reads every conforming value
under either printing; its `as_number` read is wider than the amended type and is now stated
in the code as a reader's tolerance.

## The mis-filing, met twice inside one issue

Two of Issue #152's three strikes are filed one clause late by the outline's page-straddle:
page 589's, on Table 257, prints under §12.8.2.3 where the table is §12.8.2.2's; page 802's,
on Table 380, prints under §14.8.5.4.5 where the table is §14.8.5.4.4's. ADR 0712's placement
rule was applied before any verdict was written — which is what it is for — and both rows'
notes now name the straddle so the next reader does not re-derive it.

## What decays

Five issues gain verdicts and leave the population: the two heads' and the three the live head
carried. The base count for the next use is the greps' answer, not this file's.
