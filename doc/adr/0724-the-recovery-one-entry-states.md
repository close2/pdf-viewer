# 0724 — The recovery one entry states, and the grammar a reason had confused with a registry

Status: accepted.
Context: the errata selection rule's eleventh use — the first time both rankings topped out in
plateaus of one-issue rows, the second consecutive use whose base count reproduced the previous
use's closing arithmetic, and the first whose paying row sat a rank below the live head.

## The rule, unchanged

ADR 0627's rule with ADR 0637's repair to step 2, ADR 0653's tie-break, ADR 0671's fourth step,
ADR 0691's writing rule and ADR 0712's placement rule:

> Rank each ledger row by the errata annotations that fall on it whose issue number this tree
> names nowhere. Rank once over the live rows and once over **every** row, take the head of the
> two, and prefer the settled row where they tie. Reassemble the issue from every clause `emit`
> files it under, and read the issue whole — and a verdict written under a heading is a claim
> about a page, not about a clause, until the rectangle has been placed.

Of the issue numbers in `doc/ISO_32000-2_sponsored_EC3.pdf` that carry a strike or a caret
under the recipe's own single-issue line parse, **99 were named nowhere at this round's base**,
of a population of 302 — the tenth use's closing arithmetic, 104 less its five verdicts,
reproduced by the greps rather than quoted from the record, for the second consecutive use. A
parse that also reads the multi-issue annotation lines counts 310 and 101, which are likewise
the tenth's figures less the same five.

## The heads: two plateaus, and the work one rank below both

Over **every** row the head is a settled pair tied at five — **§7.5.4, `implemented`, and
§13.6.3.1, `out-of-scope`** — and over live rows a four-way tie at five: §7.5.5, §12.5.6.5,
§12.7.5.5 and §14.7.2. Step 4 takes the settled pair; the eighth use's practice follows — the
head to a verdict, then downward until a row pays.

**Both settled heads confirmed their rows and paid nothing.** §13.6.3.1's five annotations
(Issues #18 and #362 — a `DefaultRGB` routing sentence for 3D streams, and `/Resources`
deprecated) sit inside `CLAUDE.md`'s multimedia exclusion end to end. §7.5.4's five were three
of Issue #109 — example typography, ten annotations across four clause families, nothing
normative — and two of **Issue #272, whose strike is not §7.5.4's at all**: the rectangle places
it on §7.5.2's binary-marker sentence, one clause before the outline's heading for the page. The
amended sentence — the header line shall be immediately followed by *a line containing only a
comment that starts with* at least four binary characters — binds whoever writes a whole file,
and this tree writes §7.5.6's appends and never a header. ADR 0712's placement rule was applied
before any verdict was written, for the third consecutive use.

**The live plateau confirmed four times over.** Issue #159 rewrites Table 236's `/DigestMethod`
into an *unordered* array whose values are each one of five names — inside the seed value
dictionary §12.7.5.5 already declines whole. Issue #17's inserted NOTE states that a link's
activation area and appearance need not coincide, which is `link.rs`'s construction exactly
(`/QuadPoints` bounds activation, the border stays §12.5.4's rectangle); Issue #299 moves two
`/BS` version markers to PDF 1.3, and a version marker binds a producer. Issue #522 adds an
informative NOTE that `/Size` does not decrease in incremental updates, beside a departure this
tree measures about understatement. Issues #396 and #93 give Table 354's `/Namespaces` and
Table 355's `/NS` writer-side completeness rules and deprecate `/R` — and the role walk takes
each element's own `/NS`, never the root's array.

## The payment: Table 29's `/Lang`, and an invalid identifier is unknown

**Issue #105 is one caret and it changes what this reader answers.** It inserts *or invalid
(see 14.9.2, "Natural language specification")* into the entry's last sentence: if this entry
is absent or invalid, the language shall be considered unknown. §14.9.2.2 defines what a
language identifier is — "either be the empty text string, to indicate that the language is
unknown, or a Language-Tag as defined in BCP 47" — and this reader handled the empty string and
absence while carrying every other value to every consumer as if it named a language. A catalog
`/Lang` holding prose — a real producer's shape — reached the accessibility tree as an
identifier no locale matches.

`structure::document_language` now answers `None` for a tag that fails BCP 47's grammar.
Two boundaries are deliberate and both are documented in place:

- **Well-formedness, not validity.** BCP 47 defines `Language-Tag` twice over — RFC 5646
  section 2.1's grammar, and a validity judgement against the IANA subtag registry. The grammar is
  self-contained and is what "as defined in" pins; the registry is a moving list this program
  does not hold, and a well-formed unassigned tag still names a language to the file's own
  reader in a way prose does not. §14.9.2.2's row had declined any parse on a sentence that
  conflated these two judgements; the reason is retired and the half of it that was right — the
  registry — is still not consulted.
- **The catalog entry alone.** Table 29's entry is the one place the standard states the
  recovery. An element's or a `Span`'s invalid tag is still carried as the file writes it:
  no clause states a reader's recovery there, and inventing "unknown" for an element would
  also cancel §14.9.2.3's inheritance for it.

`an_invalid_catalog_language_is_unknown` pins the end-to-end answer and
`a_language_tag_is_judged_by_bcp_47s_grammar` walks the grammar in both directions —
grandfathered tags and private tails included, because refusing what the production names
would be a second defect. Calibrated per trap 13, above the round's commit: the no-validation
plant passes every pre-existing accessibility test and fails only the new end-to-end test; the
reject-everything plant fails the older tests that assert a valid tag is carried. No corpus
figure moves — the corpus gate and the oracle print the same numbers either way.

## What this adds to the rule's record

- **A row's stated reason can overstate a requirement's cost, and no sweep reads a reason's
  internal logic.** The settled-row mechanism's six recorded shapes are all evidence weaker
  than a claim; this is a *live* row whose disposition was sound and whose argument was not —
  two judgements under one word, with the cheap one declined at the dear one's price. The only
  instrument that puts a round in front of such a sentence is this ranking.
- **A head plateau of one-issue rows is what the decayed rows' absence looks like, and the
  third use's tie-break already prices it — but only inside a tie.** The ranking's unit is the
  annotation, so five repetitions of one substitution weigh five while four distinct issues
  weigh four. This round the whole plateau was the first shape and the work was the second,
  one rank down. Not a rule change: reading a plateau out is cheap, its issues leave the
  population — fourteen this round, the rule's largest single decay — and the next requirement
  surfaces.
- **The base count agreed with the derivation for the second consecutive use**, which is what
  the eighth use's off-by-one and the ninth's verification rule were for. The greps remain the
  instrument; the record remains a derivation.
