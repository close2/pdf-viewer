# 2051 — The identification namespace is `http` for both parts, as erratum #123 corrects part 4, and a 3D annotation outside PDF/A-4e is the forbidden-subtype loss

Session 1607, the archive slot of batch ninety-six. Status: **accepted**, **built** in
`pdf-transform` and `pdf-archive`. Code: `crates/pdf-transform/src/archive/prepare.rs`
(`identification_uri`, `prepare_forbidden_annotations`), `crates/pdf-archive/src/errata.rs`,
`crates/pdf-archive/src/table/metadata.rs` (`IDENTIFICATION_URIS`), `crates/pdf-archive/src/table/
interaction.rs` (`annotations_outside_the_engineering_flavour`). Section 4 is the census's next
part 4 site, built because the budget allowed it.

## 1. What the standard states

ISO 19005-4:2020 section 6.7.3, as published (and as the SRPS 2023 adoption this project holds
prints it), gives the PDF/A identification schema's namespace URI with an `https` scheme; ISO
19005-2 section 6.6.4 gives it with `http`. The PDF Association's errata for ISO 19005-4:2020, issue
123, marked ISO-approved, deletes the `s` and adds a note that an implementation may choose to accept
`https` in addition to `http`; the same issue corrects Table 2's `pdfa:conformance` to
`pdfaid:conformance`. `pdf_archive::errata` already carried the issue, attached to a part 2 row.

## 2. Decision

**The writer writes `http` for every target**, because that is what the corrected text states; the
note's permission is a reader's, not a writer's. Before this, a part 4 output used `https` and
failed veraPDF's 6.7.3-1 alone (round 1599's sixth witness). **The validator keeps accepting both
spellings for both parts**, now on the erratum's own note rather than on the printed inconsistency,
and a conversion still removes both before writing. Erratum #123 moves from the part 2 row
`metadata/identification-conformance-level` to the four part 4 rows that read the schema — the part
number, and the three flavour rows for the prefix — and a test holds every erratum to a row that binds
a part 4 target. The module comment's "departure from the letter of one cell" for the `pdfa` prefix is
gone: the corrected cell is what the rows read. veraPDF agreeing is evidence about the reading, not
its ground.

## 3. Evidence

The output-intent witnesses `6-2-2-t02-fail-a` (round 1599's) and `6-2-3-t01-fail-b`, and
`6-6-2-3-3-t03-fail-b`, converted to 4: veraPDF
1.30.3 not compliant before (6.7.3-1), compliant after; this validator conforms both times; pages
byte-identical at 72 and 720 dpi.

## 4. `annotations/three-dimensional-only-in-engineering-files` — the subtype rows' loss

ISO 19005-4 section 6.3.1 admits `3D` and `RichMedia` "only ... in a PDF/A-4e compliant file". At
PDF/A-4 and PDF/A-4f the subtype is therefore one the target does not admit, which is what
`annotations/subtype-defined-in-iso-32000-2` already answers with `Loss::ForbiddenAnnotation` and
`Rewrite::ForbiddenAnnotationRemoved` (ADR 1099), the loss's own sentence already naming 3D artwork.
So the site takes the same answer and the same removal: `annotations_outside_the_engineering_flavour`
is the row's population in `ForbiddenSubtype`'s shape, chained into the one the removal walks, and
the site joins `PRESERVABLE_BY_PAGE` and `SUBTYPE_REQUIREMENTS` so that `preserve` keeps the
producer's poster on its page (ADR 1123). The first answer is still the flavour, and the loss's
sentence now says PDF/A-4e keeps the annotation whole. **One population means one answer for the
page's marks**: where a page holds a `Screen` and a `3D`, a `preserve` at either row keeps both
posters; that is more than a `discard` at the other row asked for and never less, which ADR 1942's
lower bound permits. **`FileAttachment` is not built**: removing the annotation takes the file its
`/FS` names, and keeping that file in the name tree is a rewrite of its own.

Witnesses: veraPDF's `PDF_A-4/6-3-1-t01-fail-d` and `-e`, refused before; with `--authorise
forbidden-annotation` (and `embedded-file` for `-e`, whose RichMedia assets are a second loss) both
convert to 4, veraPDF 1.30.3 compliant, the poster gone from the page as the loss says; under
`as-shown`'s `preserve` the poster is drawn where it was.
