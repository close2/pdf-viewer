# 0681 — The operator category a row said we could not run

Status: accepted.
Context: the successor selection rule's sixth use, its second run with the fourth step in place, and
the first time the full ranking's head was a row whose defect was a *denial* rather than a gate.

## The rule, unchanged

ADR 0627's rule with ADR 0637's repair to step 2, ADR 0653's tie-break and ADR 0671's fourth step:

> Rank each ledger row by the errata annotations that fall on it whose issue number this tree names
> nowhere. Rank once over the live rows and once over **every** row, take the head of the two, and
> prefer the settled row where they tie. Reassemble the issue from every clause `emit` files it
> under, and read the issue whole.

Nothing about the instrument needed correcting this time, which is the second use in a row that can
say so. Of the 307 issue numbers carrying a strike or a caret, 124 are named nowhere in this tree.

## The head

Over live rows: **§14.8.5.3, seven annotations under #224 and #357** — written bare on purpose, as
746 and 755 wrote them: neither has a verdict in `doc/errata-read.md`, and the `Issue #` form would
take both out of step 2's population without one, which is the failure ADR 0660 recorded. The same
plateau
the third, fourth and fifth uses each left standing, because none of those three took its row from
the live list. That is worth naming rather than passing over: the live ranking's head has not moved
in four uses, and the reason is not that the rule has stopped decaying. It is that the full ranking
outranks it every time.

Over every row: **§9.6.4, eleven annotations under Issue #43, Issue #111, Issue #144 and Issue
#553**, `implemented`; §7.4.1 with eight is second. ADR 0660 measured both of those from outside at
exactly 11 and 8 before the fourth step existed, and reproducing the two figures is what said this
round's arithmetic was right before it was trusted.

The tie-break did not have to run — eleven against eight is not a tie — but ADR 0653's ordering
would have chosen the same row anyway: three of §9.6.4's four issues change what the clause says,
and the fourth is grammar.

## What the issues said

`doc/errata-read.md` has all four with the rectangle that places each. In short:

- **Issue #111** strikes *operators* out of "In Type 3 fonts, glyphs shall be defined by streams of
  PDF graphics operators" and writes *objects*; inserts a NOTE 2 saying a Type 3 glyph "can use any
  PDF operator from any operator category … subject to additional restrictions described in this
  clause"; and inserts a paragraph requiring an implementation to avoid infinite recursion where a
  glyph description refers to itself, the result being implementation-dependent.
- **Issue #43** inserts *The number*, *The numbers* and *the numbers* six times ahead of Table 111's
  symbol glosses. Grammar.
- **Issue #144** corrects the clause's EXAMPLE from `/LastChar 104` to `98`.
- **Issue #553** adds a normative reference to Adobe Technical Note #5902 and, by the page-straddle,
  a §9.6.3 caret about deriving an instance font's PostScript name. A writer's rule.

The recursion paragraph is already `draw_type3_glyph`'s `MAX_FORM_DEPTH`, which was written from
principle 3's budgets rather than from the clause; the clause states it now, and this program's
implementation-dependent result is *reported* where the standard permits anything at all. That is
the same shape as Issue #154's mitre floor: a clause that says a thing is a stronger answer than one
that implies it.

## The decision

**§9.6.4's row claimed a gap that has not existed since the eleventh session, and the erratum is
what found it.** The sentence, standing since the tenth:

> A glyph description whose marks are an inline image draws nothing yet and reports, which is
> §8.9.7's gap rather than this one's: 10 corpus documents are in that position.

`pdf_model::inline_image` landed in the eleventh (ADR 0019) and §8.9.7 has been `implemented` since,
so there was never a gap to attribute a refusal to. Measured: a `d0` description's inline image is
drawn, at the matrix the description's own `cm` gives it, and nothing is reported. All three of the
sentence's claims were false, and Issue #111's NOTE 2 is precisely the sentence that states the
permission the row denied — Table 50's categories include inline images.

So:

1. The sentence is replaced by what is true, with the two tests that hold it.
2. **The corpus figure is withdrawn rather than re-derived.** It counted documents exercising a debt
   that does not exist; a population of files taking a working path is not a ledger fact, and
   re-deriving it would need a census example and a corpus pass for a number nothing would use.
3. Two tests are added to `crates/pdf-model/tests/type3.rs`:
   `an_inline_image_is_a_glyph_description_s_marks_like_any_other_operator` and
   `a_d1_glyph_description_drops_an_image_and_keeps_an_image_mask`. The second exists because the
   first alone would leave "images are dropped inside a glyph description" as a live hypothesis: the
   `d1` case *is* a drop, required by Table 111 and §8.6.8, and the image-mask exception is the only
   thing that distinguishes the rule from the defect.
4. `type3.rs`'s module comment keeps its quotation of the published sentence — `doc/md/` is what the
   quotation gate reads — and names the erratum that retires its last word.

**No behaviour moves.** This round adds evidence and removes a false claim; no pixel changes.

## Why nothing could have printed it

The sentence is a row's *denial* of a capability, and:

- **`--bin overstated`, the eighteenth sweep, reads the inverse.** It finds a row claiming a thing is
  read against a descendant's denial that anybody reads it. This is a row *denying* a capability
  under a status of `implemented`, with the capability living in another clause family entirely —
  §8.9.7 rather than §9.6.\*, so no parent-and-child relation connects the two sentences.
- **`--bin blockers` reads a blocker's clause**, and this sentence names §8.9.7's *gap*, which is
  prose about a clause whose row is settled. The blocker sweep would have printed it if the phrase
  had been in its vocabulary; ADR 0475's population is a stated blocker rather than an aside.
- **`spec-errata check` cannot see the erratum that leads to it.** The strike is one word.

What did print it is the ranking, indirectly and by construction: it put a round on the clause, and
the clause's own new NOTE named the category.

## Two things about the instruments

**An erratum's *added* text cannot be a rustdoc blockquote, and that is not a defect in the gate.**
`every_quotation_is_the_standards_own_words` reads every blockquote under `crates/` and asks
`doc/md/` for it; an inserted sentence is in no clause of that conversion, so the gate fails with
*§9.6.4 does not contain … as written* — which is exactly right, because the alternative is a gate
that cannot tell a paraphrase from an amendment. The convention that already answers it is
`measurement.rs`'s for Issue #534: an erratum's replacement goes in *italics*, naming the issue. The
rule was written down for *struck* text and not for added text, and now it is.

**Trap 13 sprang on this round's own calibration, in its own words.** The transpose plant — swapping
`b` and `c` of the glyph's transform — passed, because the font matrix composed with the text
rendering matrix is diagonal and a diagonal matrix agrees with its own transpose. The fixture's
description states `750 0 200 375 0 0 cm` now; the shear makes the placed matrix disagree with its
transpose and the same plant fails. A rectangle is not asymmetric enough.

## Consequences

- §9.6.4's row is true and carries two more tests; §9.6.3's records Issue #553.
- Four issue numbers leave the unread population, so the next use of the rule sees a different head
  over settled rows.
- The live ranking's head is §14.8.5.3 for the fourth use running. A round that wants it will have
  to take it deliberately, because the full ranking has out-ranked it every time the fourth step has
  been run.
