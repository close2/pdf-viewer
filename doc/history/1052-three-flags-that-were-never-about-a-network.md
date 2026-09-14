# 1052 — three flags that were never about a network

Date: 2026-09-14. ADR 1066. Touched `pdf-model` (`submission.rs`, `forms_data.rs`), `pdf-syntax` (`xref.rs`), two test files,
four ledger rows. Continues 1047 and ADR 1062, which left §12.7.6.2 `partial` on five Table 240 flags that reported rather
than applied.

**Read one at a time, three of the five had nothing to do with the verb the clause was refused for.** Bit 7 wants Table 246's
`/Differences`, "[a] stream containing all the bytes in all incremental updates made to the underlying PDF document since it
was opened", and the same row *requires* the save that makes them — "[a]n incremental update shall be automatically performed
just before the submission takes place". That is `ViewState::save`, which bit 9 already submits whole. Bit 14 wants the `/F`
to be "a file specification containing an embedded file stream representing the PDF file from which the FDF is being
submitted": the same bytes, in §7.11.3's dictionary around §7.11.4's stream. Bit 8 wants "all markup annotations in the
underlying PDF document", which Table 171's third column enumerates and §12.7.8.3.4 asks be written with Table 254's ordinal
`/Page` — including the ones a person added this session, because that same save is what puts them in the document. All three
are composed now, and **bit 12's `ExclFKey` with them**: it was "met" by an absence and is applied against an entry that
exists. Two gaps inside them report rather than hide — an annotation entry naming an object of this document, `/AP` and
`/Popup` above all, cannot travel into a file with no object space of this document's, and Table 43's `/F` is "[a] file
specification string of the form described in 7.11.2", a path this crate has none of by construction.

**Bit 11 stayed, and for the table's words rather than for want of code.** It narrows bit 8 to the annotations whose `/T`
"matches the name of the current user, as determined by the remote server to which the form is being submitted" — the
server's determination, and this program has no name of its own either, since `add_markup` writes no `/T` for exactly that
reason. The two ways to be wrong are not symmetric: including a non-match breaks bit 11's own *only*, and including none
breaks nothing the table states, because bit 11 *is* the narrowing. So it is applied whole and the sentence goes to the host.
Bit 10 stays owed on its own NOTE 1, which puts the date fields in ECMAScript.

**The composed FDF states §7.5.4's table although §12.7.8.2.1 calls it optional**, and bit 14 is what forced that: a body
carrying a whole PDF carries that file's own `obj` keywords. **Which found a defect one crate down.** `xref::header_position`
preferred `%PDF-` and fell back to `%FDF-`, so an FDF with a PDF embedded a few hundred bytes in was measured from the
*embedded* file's header, every offset came out short, the document was recovered by scanning — and the scan found the
embedded file's objects. The earlier marker is the header now; of 976 corpus documents none has `%FDF-` before `%PDF-` in its
first kilobyte, so nothing else moves.

Rows: §12.7.6.2 stays `partial` with two flags left instead of five; §12.7.8, §12.7.8.3.4 and §7.5.2 record what each gained.
Calibrated (trap 13): eight planted defects — the header ranked by kind, bit 7's stream dropped, bit 8 taking every
annotation, its ordinal pinned to zero, the unportable entries kept, bit 11 narrowing nothing, bit 14 embedding nothing, bit
12 ignored — each named by its own test and no other. No page moves: nothing here is drawn.
