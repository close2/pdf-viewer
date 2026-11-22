# 3359 — An annotation no reader draws is preserved as the nothing it drew, and one removal answers every subtype row

Session 2164. Status: accepted. Context: ISO 32000-2 §12.5.2 (Table 166), §12.5.3 (Table 167),
Table 171; ISO 19005-2 and ISO 19005-4 sections 6.3.1 and 6.3.3; ADRs 1123, 2051 section 4; A694
item 1; R16 of `doc/pdfa-blocking-issues.md`.

## The questions

1. `issue7446` (pdf.js) stops under `as-shown` at 2b and 4. Printed: "one of the annotations to be
   removed states no normal appearance stream — so there are no marks of the producer's to keep".
   Its one annotation states neither `/Subtype` nor `/AP`, and a `/Rect` off the page.
2. `multimedia_annotations` (pdf.js) stops under `as-shown` at 4. Printed at
   `annotations/three-dimensional-only-in-engineering-files`: "this conversion composed pages for
   other content but none for this requirement". Its `RichMedia` annotations' marks are carried, but
   filed under `annotations/subtype-defined-in-iso-32000-2`, the first of the four rows one removal
   answers.

## Decision

1. Table 166 makes `/Subtype` required, so an annotation stating none has no Table 171 type and no
   handler; Table 167's `Invisible` flag then has a reader render it "using an appearance stream
   specified by its appearance dictionary, if any", and it specifies none. The window drew nothing,
   and the `preserve` that keeps what the window drew keeps nothing for it:
   `appended_pages::drew_nothing` passes it over, a row whose every removed annotation is such
   answers `Configured` with the removal and no page, and the appearance rows that annotation
   fails take the subtype row's configured answer. An annotation with a type and no appearance
   keeps the refusal: this viewer may construct one for it.
2. Marks carried under any of the four subtype rows answer each of them (ADR 2051 section 4: one
   removal takes the annotations all four report).

## What it costs

`issue7446` loses an annotation and its `/Contents` string, nothing drawn; it converts at 2b and 4
under both shipped profiles, page 1 identical, veraPDF 1.30.3 compliant. `multimedia_annotations`
converts at 4 under `as-shown`, veraPDF compliant.
