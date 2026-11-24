# 3557 — Under `as-if-printed` a page holding a content repair section 5.1 refuses is photographed

Session 2214. Status: **accepted and built**; extends ADR 2516 and ADR 3543. Context: owner answer
A694 (items 1 and 3), A707, the owner's PDF/A stance of 2026-10-27 ("a company must archive every
PDF it can see — the elegant remedy first, else a stated-loss appended page; no refusal is
absolute"); ADR 3521 (section 5.1 answered by what the repair was), ADR 3541 (the classes).
Clauses: ISO 19005-2 and ISO 19005-4 section 5.1; ISO 32000-2 §7.8.2, §8.4.2, §8.6.8, §8.9.7, §9.4.1.

## Context

Round 2210 put `pdf_model`'s content classes on the channel, and seventeen documents of the
census were then refused at section 5.1 under both profiles by a content class: ADR 3543 had a
content class "answered where the page is photographed", but nothing asked for the photograph. ADR 2516's fence asks for one only
for the rows `archive/fenced.rs` lists, and section 5.1 was not one of them.

## Decision

1. **Section 5.1 is a fenced row for the repairs it judges inside content.** `fenced::pages` adds
   the pages each such repair sits on (`content_repair_pages`): the page whose `/Contents` held it,
   or every page whose resources reach the form, pattern or glyph procedure that did. Where a repair
   names no holder, or its stream is one an annotation's appearance reaches (a photograph leaves
   `/Annots`), or no page reaches it, the row keeps its refusal by the class's name, as every
   fenced row does when its pages are not all known.
2. **What another rewrite writes out is not photographed for**: a salvaged run and a lone delimiter
   (`as_drawn`, ADR 2638 and 2859), an integer past 64 bits (ADR 3533), and an operator ADR 3558
   writes out where the page draws the same. The elegant remedy comes first, the photograph second.
3. **`base_standard::settled` counts a form taken with its pages**: a repair in a form, pattern or
   glyph procedure is answered where every page reaching it was photographed and no annotation
   reaches it (`fenced::taken_with_the_photographs`).
4. **The row's losses are the photographs' and whatever else section 5.1 cost**: a damaged stream
   restated keeps `damaged-data-as-decoded` beside `page-as-image` (ADR 3522); a refusal of the
   restatement stays the row's refusal.
5. `as-shown`, 2u and 2a keep ADR 2516's gate: no photograph is offered there.

## Evidence

All seventeen convert under `as-if-printed` at 2b and 4 once this and ADR 3558 are built: ten are
photographed (two of them beside operators written out), seven answered by ADR 3558 alone —
`hesr13613` among them, whose repair sits in a widget's appearance no photograph takes. veraPDF
1.30.3 passes every output.
