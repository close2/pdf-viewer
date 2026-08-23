# ADR 0513 — The vote a flat sheet does not get

Status: accepted, 2026-08-23. Session 681. Acts on the finding ADR 0499 recorded and declined to
act on. Changes what the oracle *concludes*, on 29 pages of 1794, in four directions at once.
**No pixel moves.**

## The finding, reproduced before it was built on

ADR 0499 closed with an item marked owed:

> **The gate was not changed.** A voting reference whose raster is constant is, on the evidence
> above, contributing no information to a verdict, and a condition that refused it a vote would be
> the honest instrument. It would also move pages between four lists at once, and trap 11's rule —
> a report is only as good as the condition it fires on — makes that a decision with its own ADR
> rather than a corollary of this one.

Its evidence is specific and it reproduces exactly. On `bitmap-symbol-texthuffrefinecustom.pdf`
page 1 the gate printed *mean 13.12, worst tile 144.56, differing 5.15%, ssim 0.8990* against a
bound of *1.00 / 5.00 / 1.00% / 0.9900*; `magick identify` on the two panels that carried the
verdict says `mupdf.png` and `ghostscript.png` have **one colour apiece**, white, over 399 × 400,
while ours has two and `poppler`'s has 198. `255 × (1 − 0.948543)` is 13.12, which is the mean the
gate printed. The comparison had no second operand and the verdict was a description of us.

**The reference logs are not the instrument**, and that is worth recording because it was the
obvious first idea. Of the four pages ADR 0499 names, three have `poppler`, `mupdf` and
`ghostscript` **all silent** while two of them return a blank sheet and exit 0. A rule that fired
on a renderer reporting an error would have reached one page of the four.

## What is being discriminated, and why "constant" is not the predicate

A raster of one colour arises two ways, and only one of them is a failure:

- the page's correct rendering *is* a flat sheet — an empty page, or one covered by a single
  fill. The flat raster is then a reading of the file;
- the page has marks and this renderer did not draw them.

Our own render cannot separate those without circularity in both directions: excusing a flat
reference because *we* drew nothing would forgive every page we lose a mark on, and disqualifying
one because we drew something would let any render of ours dismiss a disagreement.

The non-circular evidence is the other references, and the question to put to them is not "did
anybody draw" but "does anybody who drew **disagree**". A flat sheet inside `Tolerance` of a
raster with marks on it is the same picture as far as this instrument can measure — refusing it a
vote buys nothing and costs an agreement. A flat sheet **outside** the bound of a raster with
marks establishes that there was a picture to draw and that this renderer did not draw it.

So the rule, in `pdfref::consensus_abstentions`:

> A reference whose raster is one colour takes no part in the consensus where a reference that
> drew marks fails to agree with it — judged by the same `Tolerance::accepts` that decides every
> other agreement here. It is still measured, still printed, and still named.

Uniformity is exact rather than within a noise floor. A renderer that drew anything antialiases
its edges, so the population this separates is not near-flat rasters but rasters carrying a single
value, and no threshold had to be invented.

## What it refuses to do, and each refusal is a page

- **Where every reference is flat, nobody abstains.** Every independent reading of the file says
  the page is a flat sheet, which is a reading; a render of ours that puts marks on such a page
  stays `contradicted`. `pdfref`'s `a_page_no_reference_draws_still_contradicts_a_reader_that_draws`
  is that case, and it is the defect class the rule must not suppress.
- **Where two flat rasters disagree with each other and nothing drew, nobody abstains.**
  `bitmap-symbol-context-reuse.pdf` is `poppler` white, `ghostscript` white, `mupdf` entirely
  black; the verdict still rests on two failures agreeing, and the rule cannot reach it, because
  the only evidence that the page has marks is our own render. It stays contradicted and the
  group says so.
- **Where a flat raster is the page, the rule gets it wrong and it is named.**
  `recursiveCompositGlyf.pdf` page 1 is solid red under §9.3.6's "if the only glyphs shown have no
  outlines … no clipping shall occur"; this tree, `poppler` and `hayro` draw it, `mupdf` returns
  white, and `ghostscript` recovers the malformed composite glyph and draws letters. So the only
  renderer with *marks* is the one whose reading the clause does not support, and both flat rasters
  abstain — one a failure, one the page. `NOT_COMPARABLE_A_FLAT_SHEET_IS_THE_PAGE` holds it. Every
  candidate refinement that would rescue it reads our own render, and the three pages below where
  *ours* is the flat one are what such a refinement would hide. **A limit that is one page and
  named is better than a circularity that is invisible.**

## Which verdict the page then gets, and why not `ambiguous`

Where the abstentions leave fewer than two readings the outcome is `NotEnoughReferences`, which
this gate calls **`not comparable`**. On all 29 pages exactly one reading is left.

`ambiguous` would also have been a true sentence — the abstention's own precondition is that a
reference that drew disagrees with a flat one — and it is the wrong bucket. `doc/oracle-and-corpus.md`
§3a's whole argument is that `ambiguous` is the last population where a defect can live without a
name, and its definition invites the reading "a corner of the specification". Trap 9's fifth shape
is *shared code manufacturing the absence of a consensus*, and inside `ambiguous` it was
indistinguishable from a genuine disagreement. `not comparable` says what is true and what is
useful: **the gate has one reading of this page and cannot judge it.** That is the same move
ADR 0410 and session 579 made for `no render` and for this bucket in the first place.

## What moved, measured on the corpus rather than predicted

One run before the change and one after, same machine, same warm reference cache, 1794 pages:

| verdict | before | after | |
|---|---|---|---|
| agrees | 908 | **902** | −6 |
| contradicted | 65 | **60** | −5 |
| ambiguous | 786 | **768** | −18 |
| our geometry | 2 | 2 | — |
| reference geometry | 2 | 2 | — |
| not comparable | 13 | **42** | +29 |
| no render | 18 | 18 | — |

Every page that moved, by direction:

| from | to | pages | |
|---|---|---|---|
| `ambiguous` | `not comparable` | **19** | 18 of `AMBIGUOUS_SHARED_JBIG2_DECODER`, plus `recursiveCompositGlyf.pdf` |
| `contradicted` | `not comparable` | **4** | three of `CONTRADICTED_SHARED_JBIG2_DECODER`, plus `issue11549_reduced.pdf` |
| `agrees` | `not comparable` | **6** | `issue17333.pdf`, `issue18042.pdf` pages 1–4, `text_field_own_canvas_calc.pdf` page 3 |
| `contradicted` | `ambiguous` | **1** | `issue11740_reduced.pdf`, where only `ghostscript` was flat |

Nothing moved in the other direction: no page became `agrees`, none became `contradicted`, and no
page's verdict changed on any of the **21** further pages where a reference abstained while two
readings survived. The gate now prints that census on every run — how many pages carry an
abstention, and how many of those were left with fewer than two readings — because a rule that
refuses a vote owes the count of votes it refused.

**Six agreements were lost and they are the most interesting six.** On each, two flat sheets
outvoted a renderer that drew, and our own raster was one of the flat ones:

- `issue17333.pdf` page 1 — `mupdf` draws 0.346 of 255 and `hayro` 0.262; `poppler`, `ghostscript`
  and this tree return white;
- `issue18042.pdf` pages 1–4 — `mupdf` alone draws, at 15.9375 of 255, on a page this tree already
  reports;
- `text_field_own_canvas_calc.pdf` page 3 — `ghostscript` draws 0.3136 and `hayro` 0.2352,
  `poppler` and `mupdf` return white, and so do we.

`hayro` never votes and its agreement is never evidence, but it is a separate interpreter and its
siding with the drawing reference on two of the three is what makes these worth a look. None is
diagnosed here. What the round asserts is only that "PASS — agrees" was the wrong sentence for a
page where one renderer drew something we did not and two returned blank paper.

## Consequences

- `pdfref::is_uniform` and `pdfref::consensus_abstentions` are public and carry the argument;
  `Triangulation::abstained` names who abstained, and `report::summarise` and the oracle's per-page
  line both print it. Seven new tests in `pdfref` pin the rule *and its refusals*.
- `CONTRADICTED_SHARED_JBIG2_DECODER` is 7 → 4, `CONTRADICTED_REFERENCES_DREW_NOTHING` 2 → 0 and
  kept for its argument, `AMBIGUOUS_SHARED_JBIG2_DECODER` 19 → 1,
  `AMBIGUOUS_REFERENCE_DREW_NOTHING` 6 → 7, and four `NOT_COMPARABLE_*` groups are new.
- No clause row moves: this changes an instrument, not an implementation. §9.3.6, the one clause a
  new note cites, is `implemented` and unchanged.
- **What is owed** is the three pages of `NOT_COMPARABLE_A_MARK_ONE_REFERENCE_DRAWS` that are not
  `issue18042.pdf`, by `doc/todo/00`'s method — a page where two of five renderers place a mark and
  we do not is exactly what that method is for, and until this round the oracle called it an
  agreement.
