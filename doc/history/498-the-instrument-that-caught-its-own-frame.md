# 498 — The selection-geometry instrument, and the frame it caught on its first pass

**Finding.** ADR 0323's instrument 1 is built as designed — the frame audit, the unique-word
matcher, the two bounds re-derived by the instrument itself, the drag half in `viewer-core`'s
headless harness — and its first pass caught its own builder's frame reading: `pdftotext -bbox`
**states** the unrotated crop box as its page size while its word **coordinates** are in the
rotated, displayed frame, so a normalisation that inverted `/Rotate` (on ADR 0323 Finding 1's
sentence that poppler does not apply it) put every rotated page's words 150–500 pt from where
the reference put them. Trap 3 one level deeper: a reference's stated frame and its coordinates
can answer different questions. ADR 0333.

**Date.** 2026-08-14.
**ADR.** [0333](../adr/0333-the-selection-geometry-instrument.md).
**Touched.** `tools/pdfref/src/extract.rs` (new: the two extractors and their cache, the
invocation in the key), `tools/pdfref/src/lib.rs` (exports),
`crates/pdf-model/tests/text_extraction.rs` (the verdict and the derivation, in the binary §2's
line already runs), `crates/viewer-core/tests/headless.rs` (the drag half),
`doc/adr/0333-*` (new), `doc/todo/05-an-instrument-for-the-interactive-surface.md` (instrument 1
built; what remains of the item), this file.

## The first full run's numbers

Stated here per the round's instruction — they are **not** yet in `doc/todo/02` §2, per ADR
0323's rule that an instrument's numbers gate only once they have held across rounds. The gate
line reprints the verdict; `PDFVIEWER_SELECTION_SPREAD=1` on the same binary reprints the
derivation.

**The derivation** (poppler against mupdf, the bounds' own population): 12 778 matched word
pairs over 516 documents, 458 refused by name. Horizontal edge delta median 0.0000 pt, p90
0.0002, p99 0.1888; vertical centre median 0.57 pt / p90 1.24 / p99 2.96 (the design round's
throwaway scripts said 0.59/1.23/2.79); centre over word height p99 0.5373; height ratio median
1.2900, p90 1.7388 — Finding 3's convention-against-convention row, reproduced from committed
code. **The 0.5 pt horizontal bound rejects 0.41% of reference pairs; the half-word-height
centre bound rejects 1.23%.** Both bounds therefore sit above the references' own p90, at their
p99 — where they were designed to sit — and are confirmed over their real population before
they ever gate.

**The verdict** (ours against poppler, in §2's existing `text_extraction` line): **507 of 974
documents judged, 11 161 matched words, 98.26% in bounds (10 967), 485 of 507 documents fully
in bounds**, 11.4 s warm. Refusals, by reason, each a document off the judged set (trap 11's
arithmetic, printed by the gate itself):

| refused | reason |
|---|---|
| 292 | no words in the reference |
| 117 | no unique matches |
| 28 | no words in our readback |
| 10 | unopenable to this tree |
| 8 | pdftotext refused the document |
| 6 | no page one |
| 5 | frame mismatch against pdftotext |
| 1 | pdftotext exceeded its budget |

Matched-unique fraction per judged document: median 0.80, p10 0.28 (Finding 5's segmentation
spread, printed rather than judged). The named out-of-bounds tail is small and legible:
`issue1350.pdf` (100 Type 3 words, horizontal 0.00 pt, vertical-centre convention only —
ADR 0216's box question), `issue11555.pdf` (worst horizontal 27 pt), rotated-text pages whose
convention axis lies on frame x, and a handful of one-to-three-word documents at exactly 1.00 pt.

**The drag half** passes on its first honest run: `pdftotext`'s three longest unique words on
`doc/PDF20_AN001-BPC.pdf` page one, viewport at the page's own point size, press → drag →
release across each reference box, selection contains the word and no more than the box's
neighbourhood. (Its first *dishonest* run failed for a reason worth a sentence: the worktree was
missing the `doc/*.pdf` symlinks and the panic message blamed a missing `pdftotext` — the
message now matches what `reference_word_boxes` actually could not do only in the sense that
both are loud; the setup, not the code, was at fault.)

## Gates

Run after the final edit, in `doc/todo/02` §2's order. fmt clean. clippy silent of lints (the
only `warning:` lines are viewer-qt's cold-build gcc notes about cxx-generated code, which §2
documents as not clippy's). `cargo nextest run --workspace`: **1803 tests run: 1803 passed
(2 slow), 13 skipped** — 1800 before this round, plus the drag test in `viewer-core` and
`pdfref`'s two extraction unit tests. Doctests all ok. Corpus gate ok (the known lists,
unchanged). Oracle ok in 159.3 s: contradicted 67, ambiguous 786, with the same heads on the
ranking. `text_extraction` line ok, 4 passed in 33.3 s: the pdf.js gate at 99.3%
(24014/24193 words, the same 22 below the floor), the PDFBox gate skipping (that submodule is
not checked out in this worktree, which its own line prints), the derivation declining itself
without `PDFVIEWER_SELECTION_SPREAD`, and the new instrument printing the verdict above.
Dates (1514/1545 conforming), xmp, jpeg2000, quorra (49.5 s, same six heads) and conformance
(5 passed) all ok. No ratchet moved anywhere — the instrument prints and gates nothing, as
designed.
