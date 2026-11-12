# 2820 — A pattern is given the nothing it was read against, an associated file is dated, and Annex Q reads the painted state

Round 2000, the audit-fix round, group TR (transform and archive).
Status: **accepted and built** in `pdf-transform` (`archive/reader_resources.rs`: `assign`, `supply`;
`archive/attach.rs`: `attach`, `NO_MODIFICATION_DATE`) and `pdf-archive` (`survey.rs`: `Transparency`,
`blends`; `survey/graphics_state.rs`; `survey/operators.rs`).
It answers findings of `doc/reviews/1983-specification-audit.md` and supersedes, for the points named
below, ADR 2500 section "What is built" item 1 (tiling patterns) and ADR 1270's reading of Table 45's
`/ModDate` as omissible.
Context: ADRs 0255, 0947, 1059, 1270, 2500, 2523.
Clauses: ISO 32000-2 §7.8.3, §8.7.3.1 (Table 74), §7.11.4.1 (Table 45), §14.13.2, §11.3.5, Annex Q.2.

## 1. A tiling pattern that states no `/Resources` is given an empty dictionary

ADR 2500 gave every form, tiling pattern and appearance that states no `/Resources` the page's
dictionary, on the claim that §7.8.3 ends the reader's search there. For a pattern it does not: the
fallback sentence is "All resources that are referenced from those forms and fonts shall be inherited
from the resource dictionary of the page on which they are used", and ADR 1059 reads a pattern cell
against nothing for that reason. Writing the page's dictionary into the pattern resolved names the
reader left unresolved, and the audit's fixture published 57 604 dark pixels where the source draws
four. `assign` now gives a tiling pattern an empty dictionary — the one it was read against — and
`supply` files each name the cell used into that dictionary as the empty resource of its category, so
the cell paints the nothing it painted. A name of a category with no empty resource (a pattern, a
shading, a colour space) stays refused by ADR 2500's sentence. Forms and appearances are unchanged.

## 2. An associated file's stream is dated, or nothing is attached

Table 45 makes `ModDate` "( Optional, required in the case of an embedded file stream used as an
associated file )". ADR 1270 carried the document's `/ModDate` where it stated one and omitted the entry
otherwise, reading only §14.13.2's *should*. Every file `archive::attach` keeps is associated with the
catalog, so the entry is required. The date is still taken from the file and never from a clock: a
document whose document information dictionary states no §7.9.4 `ModDate` gets no attachment, and the
route answers `NotBuiltYet(NO_MODIFICATION_DATE)`, which the profile's on-failure alternative then
handles like any refused remedy. Reading the packet's `xmp:ModifyDate` as a second source is the
construction that would lift it.

## 3. Annex Q.2 reads the state of what is painted

Q.2: "For each graphical element to be rendered on the page, the graphic state shall be checked". The
survey marked a page the moment a `gs` set a transparent state, so `q /GS0 gs Q 0 0 1 1 re f` — a state
restored before anything was painted — marked an opaque page. The four conditions now travel with the
walk's graphics state (`Transparency`), are saved and restored by `q`/`Q`, are set per key by each `gs`
(§8.4.5: a dictionary sets the parameters it names), and mark the page at a rendering operator — a
path-painting operator, a text-showing operator, `Do`, `sh` or an inline image. And an array `BM` is
judged by the mode it selects, §11.3.5: "a PDF processor shall use the first blend mode in the array
that it recognizes (or Normal if it recognizes none of them)", so `[/Normal /Multiply]` is `Normal`.

## Consequences

The audit's `pattern_resource_audit` (part 2 and part 4), `associated_files_date_audit` and the two Annex
Q survey regressions pass. A corpus document that reaches the attach route with no `/ModDate` now
converts only where its profile's alternative takes the loss.
