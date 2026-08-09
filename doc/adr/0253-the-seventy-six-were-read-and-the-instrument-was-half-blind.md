# ADR 0253 — The seventy-six were read, and the instrument that named them was half blind

Status: accepted, 2026-08-09 (session 417).

## Context

ADR 0252 found that the sponsored copies of ISO 32000-2 and the two technical specifications record
Errata Collection 3 as review markup and apply it to nothing, so `doc/md/` — the conversion the
conformance gate verifies every quotation and citation against — presents retired sentences as the
standard's current words. It built `tools/spec-errata`, fixed three quotations, and left **79
struck passages of four words or more** that `doc/md/` still carries. `doc/todo/48` made reading
the other 76 the next step, and stated the test that would close the item: *if reading them turns
up no clause this tree implements differently, the errata are a documentation concern.*

They were read. The item does not close, and the reason it does not is worse than the one the todo
file anticipated.

## What the reading found

`doc/errata-read.md` carries all 79 with a verdict apiece. Four findings and one correction to the
instrument:

**1. `/BM` was being ignored on a stored appearance stream, on the authority of a retired
sentence.** §12.5.2 used to close with "A PDF reader shall render the appearance dictionary without
regard to any other keys and values in the annotation dictionary and shall ignore the values of the
C, IC, Border, BS, BE, **BM**, CA, ca, H, DA, Q, DS, LE, LL, LLE, and Sy keys", and both
`appearance.rs`'s module documentation and the §12.5.2 ledger row quote it as the rule that shapes
the whole module. EC3 replaces the first half with "When rendering the appearance dictionary, a PDF
reader", strikes `BM` out of the list, and inserts `MK` — three marks on page 485, `/State`
`Review` `Completed`, whose positions were read off the annotations' own quadrilaterals against the
page's word boxes rather than inferred.

What the erratum removes is the only sentence in the standard that said `/BM` did not apply to a
stored appearance. §12.5.5 says the appearance's group "shall be composited … using the values of
the BM, ca and CA entries in the annotation dictionary", and Table 166's `/BM` row states the mode
for "painting the annotation onto the page" with no condition attached — unlike `/CA` and `/ca`,
which the same table qualifies with "when regenerating the annotation's appearance stream" and
"shall not be used if the annotation has an appearance stream". So the *ledger's own recorded
contradiction* between §12.5.2 and §12.5.5 is half settled by an erratum this project could not
read: for `/BM` there is no longer a contradiction, and for the two opacities there still is.

`annotation::blend_mode` now reads `/BM` on both the stored and the constructed path.

**2. The marked-content property list's key is `/MCAF`, and this tree read `/AF`.** §14.13.5's 2020
sentence named no key at all — it said only that the property list "shall specify an array of file
specification dictionaries" — and §14.13.10's own example writes `/AF /NamedAF BDC` without showing
what `/NamedAF` resolves to. So `AF` was an inference from the *tag*, and under EC3 (Issue #374) a
conforming PDF 2.0 file states `/MCAF` and this tree returned an empty list with no report. Both
keys are read now, `/MCAF` first. Table 409a, which the erratum points at, is in neither `doc/md/`
nor the annotations, so whether `/AF` should now be refused there is left open rather than guessed.

**3. §9.6.2.2's `shall` about the standard 14 fonts is gone.** "These fonts, or their font metrics
and suitable substitution fonts, shall be available to the PDF processor." is struck outright, the
paragraph above it becomes an informative NOTE with its own `shall` softened to "are required to",
and §9.6.2.1's "For compatibility reasons PDF processors shall provide glyph widths and font
descriptor data for those standard fonts" is replaced by a cross-reference. Three doc comments —
`pdf-font`'s `standard` module, its font loader and `pdf-model`'s font resolution — quoted the
struck sentence as this program's warrant for compiling the fourteen in. The warrant is now
Table 109's "(Required; optional in PDF 1.0-1.7 for the standard 14 fonts)" and §6.3.2.2's
obligation to render the page: a file may state a Type 1 font by name alone, so a processor without
metrics cannot lay out one line of it. That is a better justification than the one it replaces,
because it does not rest on a sentence about what a processor happens to have.

**4. §8.9.5.4's algorithm is rewritten and this tree implements the retired one.** Three
divergences, each in the erratum's own words: "Alternates that have no OC entry shall not be shown"
(this tree selects exactly those), "that OC in the image dictionary shall not be examined" (the
inverse of the sentence this tree reads), and a new final step, "If steps c and d above do not
identify an alternate to be rendered then the base image shall be rendered" (this tree shows
nothing). It is **not** corrected here. The amended step a) ends "then nothing shall be shown",
which reads as terminal and would leave the amended d)'s alternate selection unreachable for a
hidden base — a rewrite from that would trade one contradiction for another, which is exactly what
the 30-line argument in `alternate_image` exists to avoid. It is recorded with the carets' text in
the doc comment, the ledger row and `doc/errata-read.md`. No corpus document states `/Alternates`.

## Decision — the instrument compares with the spaces taken out

**`spec-errata`'s comparison was `conformance::quote::normalise`, and that was the wrong choice for
this crate even though it is the right one for the gate.**

The gate compares a *doc comment somebody typed* against a conversion, and there whitespace really
is noise of a knowable kind: a wrap at 96 columns against a reflow. This crate compares **two
extractions of the same glyphs by different programs**. Neither can recover a space the file does
not state, because PDF positions glyphs rather than words — the gap between two glyphs of a `Tj` is
whatever `Tc`, `Tz` and the next `Td` make it — so one extraction writes `inthe` where the other
writes `in the`, and a containment test that keeps whitespace reports a passage absent that is
present in full.

Measured over all fourteen documents: **79 struck passages found with whitespace kept, 151 with it
removed.** The comparison was blind to 72 of them, and one of the 72 is finding 1 above — the
sentence both `crate::appearance` and `ledger.toml` were quoting as §12.5.2's live rule.

So `squeezed` — normalise, then drop every space — is used for both of this crate's questions. It
can only make the comparison coarser, and it cannot make a false positive worth reading: four words
is twenty-odd characters, and two different sentences do not agree on twenty characters by putting
their spaces in different places. `Landing::in_clause` does the separating either way, and the
landings it produced are the evidence: seventeen instead of ten, four of them in-clause instead of
three.

**Finding 1 arrived in the *other* bucket, and that is worth knowing about the discriminator.**
`in_clause` compares a doc comment's cited clause against the clause the *outline* puts the
erratum's page in, and §12.5.2's closing sentence is the last paragraph of page 485 — a page whose
outline entry is §12.5.3. So the quotation cites the clause it is in, the tool called it a
coincidence, and only reading it settled that. The coarser comparison added seven landings, and
**three of the passages behind them fall across a page boundary** — §12.5.2's on a page the outline
files under §12.5.3, §9.6.2.2's under §9.6.2.3, §14.8.6.1's under §14.8.6.2. All three were
findings; the other four were the repeated "; shall be an indirect reference" that ADR 0252 had
already read. **The bucket is a sort order, not a verdict**, and a round that reads only the first
list will miss exactly the ones a clause heading straddles a page break for.

**A second correction, smaller and of the same kind.** `Note::states` collected `/State` and not
`/StateModel`, so five errata printed as "Accepted, Unmarked" and the second word looked like a
second opinion. Table 174 states *two* state models: `Unmarked` is the **Marked** model's default
and says only that nobody ticked the note off. States now print as `StateModel/State`, which
Table 175 makes always available — `/StateModel` is "[r]equired if State is present".

## What the `/State` question settled

The round was asked what to do with an erratum that is `Rejected` or unmarked, on the ground that
quoting a sentence a *rejected* erratum struck would be the mirror-image mistake. Counted: **no
note in any of the fourteen documents carries `Rejected`, `Cancelled` or `None`.** Every one is
`Completed` (827) or `Accepted` (265). Of the strikeouts of four words or more, 142 are `Completed`
and 45 `Accepted`.

So session 416's `Completed` filter was narrower than the evidence and excluded nothing that was a
rejection. Both states are acted on here and the distinction is recorded rather than used:
`Accepted` is "[t]he user agrees with the change" and `Completed` is "[t]he change has been
completed", which is a position in a workflow and not a disagreement. **The mirror-image mistake
cannot be made from these files** — which is worth writing down, because a later document set may
not have that property and the filter would then be load-bearing.

## Consequences

- **`doc/todo/48`'s closing test is answered and the item stays open.** Two clauses were implemented
  differently — §12.5.2 and §14.13.5 — so the errata are a correctness concern and not a
  documentation one. The item drops nothing.
- **The list of unread passages nearly doubled**, from 76 owed to 55 further distinct ones. The
  reading was worth more than it cost and the next reading is bigger than this one was.
- **Fifteen doc comments and eleven ledger notes now say which sentence they quote and when it was
  retired**, and the tree's own count of the hazard — "37 more passages" in one ledger row and
  "the other 37" in one doc comment, both written last session — was wrong twice over and is
  corrected to 150.
- **The conformance gate still checks against `doc/md/` and nothing has changed about that.** Every
  correction here either drops a blockquote in favour of prose or keeps the retired blockquote with
  the erratum stated beside it, so 575 quotations and 6070 citations verify exactly as before. The
  one thing this session did *not* do is teach the gate about the errata; ADR 0252's argument for
  that stands and is stronger now that the extractor has been found wrong once.
