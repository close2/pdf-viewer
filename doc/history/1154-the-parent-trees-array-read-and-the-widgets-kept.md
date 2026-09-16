# 1154 — §14.7.5.4's array read through §7.3.10, and the 242 widgets kept by §14.7.5.3

2026-09-16. Files: `structure.rs`, `accessibility_census.rs`, `ledger.toml`, ADR 1151, this record.

## The parse, and the population
§14.7.5.4's value for a content stream "shall be an array"; §7.3.10 lets any value be "labelled as
an indirect object" and EXAMPLE 2 spells one `/Nums` both ways. `elements_on_page` and
`stream_owners` matched `Object::Array` on the **unresolved** entry, so an indirectly-written array
read as nothing — and **77 of the 109 corpus documents with a structure tree** write at least one
that way (recounted; 1151's sweep followed no `/Kids` and said 75), so ADR 0325's page-scoped walk
had never run on them. `parent_tree_array` resolves first; `parent_tree_entry` does not, the object
form's whole content being an identity.

## The completeness rule (ADR 1151 §5)
Table 359 requires `/StructParent` of every structural content item, so the parent tree answers for
a page only where every item states one. Where an object does not, §14.7.5.3's direction survives —
the element's `/K` `/OBJR`. **Asked for widgets alone**, because §14.8.4.7.2's Table 368 states the
only obligation of a per-annotation element ("In a tagged PDF, Form shall be used for each PDF
widget annotation that belongs to the real content of the document") where `Annot` and `Link` state
permissions. `Tree::unkeyed_widget_owners` is the search, bounded at `MAX_OBJECT_SEARCH` = 8192,
reporting the bound, visiting nothing on a page whose widgets state the key. The census now names
the two empty answers rather than counting them: `bug1978317.pdf p1` at the bound (32 768 elements
named for one page), `bug1365930.pdf p1` the `/K`-less `Document`.

## Measured, and calibrated (traps 8 and 13)
A/B in one build, per document, over all 1005: **two** moved. Whole population elements 351 324 →
227 618, four other floors with it (ADR 1151 §6), placeless 130 171 → 8 634 — all of it
`ISO-19444-1-2019-preview.pdf`'s tree counted once per page (131 884 → 8 179) plus `pr20043.pdf`'s
`Annot`, whose content item is an object the page's `/Annots` does not list. Every content-carrying
count held: own marks 191 815, caret 113 098, lines 199 568, characters 5 354 660, pages answering 2
463. Tracked controls **272** and annotations **415** held — `prefilled_f1040.pdf` keeps its 242 —
elements 4 060 → 4 059, that `Annot`. Three fixtures, each failing with its defect planted back: the
indirect `/Nums` array, the unkeyed widget below anything the parent tree named (a `Link` beside it
as control), `prefilled_f1040` at 1248/242/242 through the viewer boundary.

## Gates, under the lock, one at a time
`fmt`, `clippy -p pdf-model -p viewer-accessibility -p viewer-core --all-targets`, `nextest` (1687),
`--doc`, `conformance` (259), `accessibility_census`, `pdf-model --test corpus` (slack 0
everywhere), `raster_golden` (**held 974, moved 0 — no pixel moved**), `text_extraction` (99.69%)
and `selection_census` (98.91% / 99.90%): every one exit 0.
