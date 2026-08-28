# 0712 — The strike the outline filed one clause late

Status: accepted.
Context: the errata selection rule's ninth use, the first time its two rankings agreed at the
head on live rows, and the first time the outline's page-straddle turned out to have sent a
recorded *verdict* to the wrong clause's row.

## The rule, unchanged

ADR 0627's rule with ADR 0637's repair to step 2, ADR 0653's tie-break, ADR 0671's fourth step
and ADR 0691's writing rule:

> Rank each ledger row by the errata annotations that fall on it whose issue number this tree
> names nowhere. Rank once over the live rows and once over **every** row, take the head of the
> two, and prefer the settled row where they tie. Reassemble the issue from every clause `emit`
> files it under, and read the issue whole.

Of the issue numbers in `doc/ISO_32000-2_sponsored_EC3.pdf` that carry a strike or a caret,
**111 were named nowhere at this round's base** under the recipe's own single-issue line parse,
which reproduces the eighth use's 302-issue population exactly — the calibration the fifth use
made standard practice. The eighth use's history file wrote "110 after this round", and the one
of difference is its own arithmetic rather than a drift: one of its five issues, #96's NOTE 5
deletion, had carried a verdict in `doc/errata-read.md`'s tables since the
four-hundred-and-eighteenth session, so only four of the five newly left the population. A
parse that also reads the multi-issue annotation lines ("Issue #47 and #48"), which the
single-issue form skips, counts 307 and 112; the additional issues change no ranking this round
reads.

## The head: a tie between two live rows, settled by the third use's tie-break

Over live rows the head is **§7.6.4.1 and §7.6.6 with six annotations apiece** — the pair that
has stood since the seventh use. Over **every** row the head is the same two: §12.6.4.17 left
the ranking when the eighth use read it, and no settled row reaches six. So for the first time
the two rankings agree at the head, the fourth step's tie-break has no settled row to prefer,
and the tie between the two live rows falls to ADR 0653's rule — read the row whose errata
strike a *cell* ahead of the row whose errata substitute a word in prose.

That settles it: **§7.6.6's Issue #16 rewrites Table 27's `/Recipients` type cell** — "string or
array" becomes *byte string or array*, and the description's two remaining "string" occurrences
become *byte string* — while **§7.6.4.1's Issue #89 substitutes *a crypt filter* for "crypt
filters"** three times in prose. Both were read whole; each falls under one `emit` heading.

Both confirm their rows rather than move them. #16's entry is read by nobody — Table 27 is
§7.6.6's public-key half, and §7.6.5's family refuses the handler by name before any crypt
filter dictionary is opened — so the correction binds whoever implements §7.6.5.3's digest, and
the row now says out loud that its enumeration stops at Tables 25 and 26 for that reason. #89
makes each of the three filter names denote one filter; `crypt::crypt_filters` resolves whatever
names `/CF` states, which is wider than the amended "limited to" and is recorded in §7.6.4.1's
row as a reader's tolerance now rather than an implicit one.

## Down the ranking, and where it paid

The eighth use's practice: the head to a verdict, then downward until a row pays. At five
annotations the tie-break prefers the two settled rows. §13.6.7.3.3's Issue #645 corrects a
glyph confusion in a clause-13 table — inside `CLAUDE.md`'s multimedia exclusion end to end,
the row confirmed. §14.5's two unnamed issues are where the round's finding was, because **only
one of them is §14.5's**:

- **Issue #69 is §14.5's**: Table 350's key column is widened — each key *should be a
  second-class name*, with *(recommended), any conforming product name, or well known data
  type* — a producer's naming rule that the `inapplicable` row's disposition survives.
- **Issue #328 is §14.4's**, filed under §14.5 because page 734 opens §14.4 and reaches §14.5
  and `emit` attributes by the outline section for the page. Its two strikes delete *contents
  of the* from the first file identifier's sentence and *'s contents* from the second's, so
  both identifiers become ones *based on the PDF file at the time* rather than on its contents
  — loosened, and rightly, since §14.4's own suggested computation names the time, the location
  and the size, none of which is the contents.

## What #328 landed on

Three things stood on the struck words, and no instrument could see any of them:

1. **A gated rustdoc blockquote.** `write.rs::identify` quotes the first identifier's sentence
   as its warrant. Both strikes are under the four-word floor `spec-errata check` compares at —
   *contents of the* is three words — the third of this rule's uses to find quoted text on a
   strike below that floor, after Issue #117's and Issue #534's; Issue #181's was the same
   blindness met by running `emit` before writing.
2. **The same function's prose**, which quoted "based on the file's contents" inline as the
   meaning of what it derives.
3. **§14.4's ledger note**, which quoted the second identifier's retired wording as the reason
   deriving `/ID[1]` from the appended-to bytes conforms.

The behaviour survives in all three places: bytes of the file *are* the file at the time it was
last updated, so deriving the changing identifier from them sits inside the amended sentence as
it sat inside the published one, and the determinism argument (this crate has no clock) is
untouched. What moved is the warrant, in the comment and in the row.

## The record's own verdict was against the wrong row

The reading also found that **`doc/errata-read.md`'s recorded verdict for Issue #691 judged the
wrong clause's row**. The four-hundred-and-eighteenth session recorded it as "'such as MD5
(described in Internet RFC 1321)' struck from a NOTE about detecting a changed page. The row is
`inapplicable` and names no digest" — three claims, and the middle one placed the strike. It is
not in a NOTE and not about detecting a changed page: it is §14.4's uniqueness paragraph, a
`should` addressed to PDF writers — and the row that "names no digest" is §14.5's, while §14.4's
row is `implemented` over a writer, `identify`, that names MD5 in its own line of code. The
outline filed the strike under §14.5 and the verdict inherited the filing.

The eighth use met the same coarseness and it cost nothing, both rows being settled under one
exclusion; this is the first time it reached a *verdict*. The consequence is unchanged — the
struck sentence was a recommendation and an example, so MD5 in `identify` moves from the
standard's named example to this project's stated choice, wanted for uniqueness rather than
collision resistance — but the record now says what the strike actually is, and the rule the
eighth use wrote is sharpened by the instance: **a round whose head is a row it did not expect
reads the annotation text before the heading, and a verdict written under a heading is a claim
about a page, not about a clause, until the rectangle has been placed.**

## The other payment: a precedence rule with no test

Continuing to the four-annotation plateau's settled rows, **§14.7.6.2 (`implemented`) carries
Issue #289**, which inserts two sentences the published clause never had: *Attribute objects
included through a class and through an array of classes within the C entry may have the value
of O and NS repeated. If a given attribute is specified more than once across the attribute
objects, the later (in array order) shall take precedence.* The published text ranked `/A`'s
objects among themselves (§14.7.6.1) and `/A` against `/C`; what two class objects disagreeing
means was stated nowhere.

`Tree::attributes` has always satisfied the amendment by construction — `/C` in array order,
each class's objects in theirs, `Tree::attribute` taking the last match — but the row's one test
attached a **single class object**, which no ordering of the class route can fail. That is the
settled-row mechanism the fifth through eighth uses each found, by a fifth shape: a round trip
that could not fail, a sentence about a sibling row, a set with no closure check, a row claiming
two written forms with a test of one — and now **a rule the code satisfies by construction with
a fixture too small to exercise it**. `an_attribute_two_class_objects_state_goes_to_the_later_one`
is the evidence now: two classes, three objects, repeated `/O` as the erratum's first sentence
permits. Calibrated per trap 13 with a plant that walks the `/C` classes in reverse order: the
plant passes the older single-class test and fails the new one. Issue #305, the same row's other
unnamed issue, marks the revision numbers beside `/C`'s class names deprecated in PDF 2.0 and
softens "typically" to *possibly*; reading the pairs stays necessary whatever their status,
because an integer beside a class name is the only thing that says whether an array element is a
name or a pair.

## What decays

Seven issues gain verdicts — #16, #89, #645, #69, #328, #305, #289 — recorded in
`doc/errata-read.md`, where step 2's second grep reads them out of the population. The
four-annotation plateau's other settled rows (§12.10.4, §12.11.2, §12.5.6.1, §14.7.5.4) were
read far enough to rank and no further; their issues stay in the population on purpose, per the
fourth use's rule.
