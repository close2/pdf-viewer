# ADR 0628 — The flag row whose second sentence had no reader

Status: accepted, 2026-08-25. Session the seven-hundred-and-thirty-fourth, a clause round under
`doc/todo/01`, and the **first use of ADR 0627's selection rule**. Amends §12.5.1, §12.5.2 and
§12.5.3 in the ledger; narrows one report in `pdf-model`; corrects two comments that attributed a
condition to Table 166 that Table 166 does not state; adds
`annotations.rs::an_unknown_subtype_with_no_appearance_dictionary_draws_nothing_and_reports_nothing`
and `examples/unknown_subtype_census`. **No status moves and no pixel moves; one report is removed
and its population is two documents in the crawl and none in the corpus.** Extends ADRs 0168, 0187,
0253, 0490, 0567, 0579 and 0621.

## 1. What the rule chose, and it is not what the old one would have

ADR 0627's ranking puts §12.5.2 at the head, tied on eleven unread annotations with §12.7.5.5 and
one ahead of §12.8.1. It was taken over the other two for a reason that is about the *rule* rather
than about the clause: §12.5.2's pages are pages the seven-hundred-and-tenth session opened, with
`emit`, and recorded a count of what they left unread. So reading them tests whether the new rule
finds anything the old method — standing on the same page, running the same instrument — walked
past. That is the strongest first use available, and it is a test the rule could have failed.

It did not. `emit` files **seventeen** annotation objects under §12.5.2, carrying five issue
numbers — **#1, #22, #124, #287, #577**. ADR 0579's round recorded #577 and wrote that one of the
seventeen is named nowhere in this tree. **Three of the five are**: #1 and #124 as well, and both
are `Review`/`Completed` errata from 2022 rather than anything new.

## 2. Issue #1, and it is §12.5.1's sentence

`emit` files it under §12.5.2 because it sits on physical page 482, which §12.3.3's outline gives
that clause — the page-straddle shape ADRs 0551 and 0567 both met. The sentence is **§12.5.1's
last**: "An interactive PDF processor shall provide certain expected behaviour for all annotation
types that it does not recognise, as documented in 12.5.2, "Annotation dictionaries"."

The erratum strikes the reference and writes `12.5.5, "Appearance streams" and "Table 167 -
Annotation flags" (bit positions 1 and 2)` in its place. `doc/errata-read.md` has the arithmetic
that places both marks against `pdftotext -bbox`.

**The old pointer was not empty**, which is worth saying because it changes what the erratum is:
§12.5.2's closing rule does tell a reader to render the appearance dictionary and ignore the rest.
What it does not do is say what happens when there is no appearance dictionary — and the two the
erratum names do. Table 167's `Invisible` row is two sentences:

> Applies only to annotations which do not belong to one of the standard annotation types and for
> which no annotation handler is available. If set, do not render the unknown annotation and do not
> print it even if the Print flag is set. If clear, render such an unknown annotation using an
> appearance stream specified by its appearance dictionary, if any (see 12.5.5, "Appearance
> streams").

## 3. The half that had no reader

`annotation::decided` has applied the *set* branch since the flag was implemented, with Table 171's
list as the condition the row itself states. The *clear* branch had two halves and only one was
answered:

- **With an appearance stream**, §12.5.5's own sentence governs — "[i]f a PDF processor does not
  have native support for a particular annotation type, the PDF processor shall render the
  annotation with its normal (N) appearance" — and
  `annotations.rs::an_unknown_subtype_still_draws_its_normal_appearance` has asserted it for a long
  time. That test is the licence the erratum grants, already in the tree.
- **With no appearance dictionary**, the annotation fell through to `appearance::construct`'s final
  arm and was reported: `Refusal::NotDerivable("its clause states no geometry")`. That arm answers
  two questions with one expression — 701's shape, 716's, 720's, and now a fourth round — because
  it also catches the Table 171 subtypes whose clause genuinely states no geometry, where the
  sentence is true. For a subtype *outside* the table it is false twice over: there is no clause,
  and nothing is owed. The row's own "if any" is a condition, and a condition the file does not
  meet is a requirement met by doing nothing.

So the report claimed this reader had fallen short where no reader could draw more, which is what
trap 11 is about. `decided` now returns `Decision::Nothing` for a subtype outside Table 171 with no
stored appearance, and the two sentences of one table row are read in one place.

**An annotation stating no `/Subtype` at all is deliberately excluded from that.** Table 166 makes
the entry required, so such a file has broken a rule rather than named a type nobody recognises,
and `issue7446.pdf`'s report — whose wording ADR 0253's round fixed so it would not begin with an
empty name — stays. The guard says `!subtype.is_empty()` for that reason and the test asserts it.

## 4. The population, measured before the report was taken away

`crates/pdf-model/examples/unknown_subtype_census` is the command `doc/todo/01`'s counted-claim
rule asks for, in `free_text_census`'s shape with one addition: an argument that is a directory is
walked, so the crawl is one command and one number rather than a dozen `xargs` chunks and a dozen
partial answers.

- **0** annotations outside Table 171 in the 974, over 964 that open.
- **134 in 4 of the 65 703 crawled documents that open** — 130 `/HeaderFooter` in one document, two
  `/BJCA:Annot`, one `/CIDFontType0` and one `/CIDFontType2`.
- **132 of the 134 state a `/AP` `/N`**, so §12.5.5's half already covers them and nothing about
  them changes.
- **The two that do not are the `/CIDFontType0` and the `/CIDFontType2`** — a font dictionary's
  `/Subtype` written onto an annotation, which is a producer's mistake and not a type this reader
  is missing. They are the whole population of the report this ADR removes, in two documents no
  gate here opens.

ADR 0490's shape: the corpus is silent and the crawl has the witnesses, and the argument is the
clause's rather than the count's. The count is what makes it honest to say the change is not a
picture.

## 5. Issue #124, which moves nothing and corrected a justification

Nine annotation objects on page 483: four `StrikeOut`/`Caret` pairs striking `1`, `3`, `2` and `4`
and writing `0`, `2`, `1` and `3`, plus a caret saying the indices were corrected to be zero-based.
They are Table 166's `/AP` bullet, and `doc/errata-read.md` has the four quadrilaterals against
`pdftotext -bbox`'s four words.

**The rectangle does not move.** On `[x1 y1 x2 y2]` the one-based pairs 1↔3 and 2↔4 and the
zero-based pairs 0↔2 and 1↔3 are the same two comparisons. What reading the bullet settled is a
sentence this tree had written twice: the bullet is an **and** — its own NOTE saying it "was
changed from 'or' to 'and' in this document to match requirements in other published ISO PDF
standards (such as PDF/A)" — and `annotation::is_empty` is an **or**. `annotation.rs` and the test
`an_appearance_box_covering_no_area_draws_nothing_and_reports_nothing` both said Table 166 excused
a writer "for exactly that shape"; the excuse is the degenerate *point* alone.

Nothing drawn moves, and nothing had to: §12.5.5 scales the appearance's transformed `/BBox` onto
`/Rect`, so a scale onto no extent leaves no mark whichever dimension is zero. **The right reason
was standing beside the wrong one**, which is ADR 0620's shape — a justification that is a claim
about the standard, decaying the way a ledger row's does.

## 6. What this says about the rule

Recorded because ADR 0627 is what this round is for, and a first use is evidence either way:

- **It found something on a page a previous round had opened**, with the same instrument, which is
  the case the rule was chosen to test.
- **The finding it led to is not an erratum.** #1 is a licence; what it bought is the *reading* of
  Table 167's row against the code, and that is where the report was. ADR 0593 §1's sentence again,
  one method later: a clean erratum has still chosen where to look.
- **And #124 is the third blindness four times over.** A strikeout whose text is a single digit
  shares no sentence with anything, so `check` could not print it, and the round that read those
  pages had no reason to look at four struck digits. `emit`, read by issue rather than by page, is
  what makes them one erratum with a caret explaining itself.
