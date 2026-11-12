# 2847 — A Type 3 width reported at its font is photographed on the pages that reach it

Session 2014, a single archive round.
Status: **accepted and built** in `pdf-transform` (`archive/fenced.rs`: `type3_pages`,
`pages_reaching`, the walk `reached_from` shared with `reached_from_annotations`).
Context: ISO 19005-4 section 6.2.10.5; ADRs 0816, 2516, 2803; A694 item 1; R03 of
`doc/pdfa-blocking-issues.md`.

## 1. What the census said

Three documents (`303226`, `issue19634`, `issue19971`) stop at part 4 under `as-if-printed` at
`fonts/type3-glyph-procedures-state-their-width` alone: a glyph procedure's `d0` operand disagrees
with the font's `/Widths` (section 6.2.10.5's second paragraph; part 2 states no such row). ADR 2516
photographs a page whose content stream holds a fault the fence keeps, but `fenced::pages` admitted a
row only where every place named a page, and the validator reports this row at the font, which no
page holds. So the photograph was never offered, and the row kept the fence's sentence.

## 2. The decision

The pages are the ones whose resources reach the font: `pages_reaching` walks each page's
`/Resources` through forms, patterns, fonts and soft mask groups, the walk ADR 2803 already makes from
annotation appearances. Every object the validator named counts, past the places it lists
(`pdf_archive::Named::objects`), and a list it cut short leaves the row refused. A font an
annotation's appearance reaches keeps the refusal, as an operator there does (ADR 2803): a
photograph leaves `/Annots`. Those pages are photographed under the wide gate (`as-if-printed`,
`state-every-loss`) at ADR 2516's losses, `text-as-image` and `page-as-image`, and the font leaves the
output with their resources. The output is held to the target afterwards, so a font another root
still reaches makes the file refused rather than wrong.

No profile's choice moves: the wide gate already photographs a page for a fault the fence keeps, and
this only tells it which pages hold this one. `as-shown` keeps the fence (A694 item 1).

## 3. Not chosen

Writing the dictionary's width into the procedure's `d0` (or the procedure's into `/Widths`). §9.2.4
makes `/Widths` what advances the text position, so restating `d0` would change no mark this reader
draws, the shape of ADR 2638's `content-as-drawn`; but it edits a glyph procedure, a content stream
ADR 0816's fence closes, and no answer of the owner's opens it for this row. It would keep the text,
where the photograph takes it, and it would answer `as-shown` too; it is the question to put if the
photograph's loss is judged too high for a width.
