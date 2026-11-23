# 3410 — A named resource the exemption withdrew findings at is removed

Session 2177. Status: **accepted** and built; ADR 3394's remedy. Context: ADR 3394, ADR 1021
(`pdf_archive::reach`); ISO 19005-2 and ISO 19005-4 section 6.2.2's last sentence.

## Decision

1. **`pdf_archive` says what the exemption did.** `Judgement::withdrawn` counts the findings the
   section 6.2.2 exemption withdrew from a row; `Exempt::removable_entries` lists, by the object
   each is written in and the run of entries to its category dictionary, every named resource
   entry the cutting walk removed **whose name no content stream in the file states**, owner or
   not: every page's `/Contents`, every glyph procedure, and every stream stating a `/BBox`, a form
   `/Subtype` or a `/PatternType`. That is narrower than the exemption on purpose: a dictionary a
   page inherits is associated with its stating owner alone (A003), and a form stating no
   `/Resources` is no owner at all. The first census removed four entries from pdf.js
   `issue6541`, whose page draws a form stating no resources that names them from the page's
   dictionary, and the output then failed `named-resources-are-defined`; the list is read over
   every content stream since.
2. **The converter removes them where a finding was withdrawn.** `Rewrite::UnreferencedResourceRemoved`
   is wanted whenever any row of the source's report withdrew one, not by a decision row: the
   clause makes the resource not used for rendering, so the removal changes no mark, and the writer
   copies only what the converted document reaches, so what the entry alone reached is not
   written. The report lists each entry (`removed_unreferenced_resources`).
3. Where nothing was withdrawn nothing is removed: then no reading fails the file on the entry's
   account, and the producer's dictionary stays as written.

## Measured

`PDFBOX-4532-reduced`: `/MC0` in the page's `/Properties` is removed with the metadata stream only
it reached; all four outputs (2b and 4, both profiles) pass veraPDF 1.30.3, which failed them on
6.6.2.3.1.
