# 2445 — An appearance-only Adobe-Identity font, and a photographed page's substitutes

Status: deferred. Session 1805, the font half of the archive slot of batch one hundred and twenty.
Context: ADR 2378, ADR 2394 (its *Refused still* list), owner answer A570.

## What was asked, and what the tree and the data say

**An `Adobe-Identity` font only an annotation's appearance shows.** ADR 2378's re-keying is a
font-level edit and already reaches such a font: a planted FreeText whose `/AP /N` alone shows an
`Identity-H` `CIDFontType2` with no program and a `/ToUnicode` converts at 2b under `as-if-printed`.
The same plant with no `/ToUnicode` keeps `COMPOSITE_NOT_SUBSTITUTED`, because ADR 2394 photographs
pages and a page's content-only interpretation never selects the font. That half is unbuilt and
has no witness: of 1 477 files under pdf.js and `doc/corpora` (62 unopenable), 23 show an
unembedded `Adobe-Identity` font, every one of them in page content and none in an appearance or
in `/DR` only — a scan whose appearance route was calibrated by two plants, found both.

**A photographed page's fonts in `substituted`.** On the corpus the premise does not hold: none of
the 24 photographed conversions (pdf.js, 2b and part 4) lists a substituted font. It holds on a
plant: a page showing the unnamed font beside a Helvetica that draws only a space passes the gate
(its readback is white space), is photographed, and the report and the file's `xmpMM:History` both
say Helvetica was embedded from Liberation Sans, while the output carries no Helvetica at all —
the page's resources went with its content.

## Why deferred

Both are builds for constructions no file in either corpus contains, against a slot sized for the
three page-tree documents beside them (ADR 2444). What each needs is written down:

- **The appearance photograph** draws one appearance stream alone, through §12.5.5's matrix over
  its `/BBox`, under the owner's gate read over that stream (its readback, and the annotation's own
  `/Contents`, which a FreeText carries as text), and writes the image as the stream's content.
- **The substitute list** drops a font whose every selection was on a photographed page's content,
  and keeps one an annotation's appearance on any page, or another page, still selects; the
  entries of `Substitutes::done` need the selection (`photographed::Deferred::selected`'s form) to
  be compared, which their push sites do not record today.

Until then the first keeps its refusal by sentence and the second over-reports by one sentence on
a file no corpus holds.
