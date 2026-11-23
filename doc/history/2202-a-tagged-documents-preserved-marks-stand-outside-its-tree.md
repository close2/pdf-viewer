# 2202 — A tagged document's preserved marks stand outside its tree, as an artifact

Session 2202, an archive round in the `lane3` worktree: the owner's A1011, then round 2198's two
leftovers. ADR 3509; Q1023. Ledger rows 14.8.2.2.1 and 14.8.2.2.2 `implemented`, before and after
(14.8.2.2.2 gains the code and the test).

**A1011 (ADR 3509).** Under a profile taking `[profile] tagging-of-preserved-marks = "discard"`
(`Loss::TaggingOfPreservedMarks`, `--authorise tagging-of-preserved-marks`), at 2b and every part 4
flavour, a tagged document's annotation marks go on an appended page enclosed as §14.8.2.2.2's
generic artifact, `/Artifact BMC … EMC`, and nothing is added to the producer's tree. In a tagged
document every mark takes the appended page rather than the producer's own. The report row carries
`outside_the_tree`, a line and a JSON `losses` entry; the row's `xmpMM:History` clause names the loss.
`as-shown` and `state-every-loss` take it; `as-if-printed` discards the annotation and appends none.
Level A keeps the refusal; Level U too, asked as Q1023. An attachment drawn as pages in a tagged
document keeps a refusal of its own sentence (`DRAWN_FILE_IN_A_TAGGED_DOCUMENT`).

**The five documents.** `PDF-Sample-Document-Fully-Featured-Layout_Redacted`, `embedded_video_avi`,
`embedded_video_quicktime` convert under `as-shown` at 2b and 4, `fileAttachment_fileAttachmentAnnotation`
and `annotation-fileattachment` at 4 (both already converted at 2b); veraPDF passes all ten outputs.
Each producer page differs from the source's render only by the removed annotation, whose ink is on
the appended page (PGM at 50 dpi).

**2198's leftovers.** `tests/archive/unreached_name.rs`: an unreadable `/FontFile2`, a `/Differences`
name no route reaches, under the machine's OpenType `CFF ` face, converted with `as-shown`'s answers
at 2b and 4 — the code reaches a glyph drawing nothing at its width, under `FontFile3`. The face is
the machine's because the route under test embeds the face the window drew. Section 3 of
`doc/pdfa-blocking-issues.md` no longer lists `issue20489` as left.

**Census.** With and without the pack alike: `as-shown` corpora 2b 412 to 415, part 4 432 to 436,
pdf.js part 4 924 to 925. Kept outputs are 2198's plus those eight; sizes change for two pdfbox
attachment documents (the drawn file's new sentence) and `digitally_signed_3D_Portfolio` under
`as-shown`, whose inner tagged PDFs now keep their marks. veraPDF: 5533 written as conforming, 47
failing outputs of 15 documents, the list 2198's. Sections 1, 2 and 9 regenerated; R15 has no
document left, R16 one (`poppler-395-0-fuzzed`).

**Left.** An inner run's losses are not named in the outer report (ADR 3509); Q1023. **Trap.**
`viewer-qt`'s `played_here` tests fail until `cargo build -p pdf-media-worker --bins`.

**Gates.** Over `-p pdf-archive -p pdf-transform -p viewer-qt`: fmt 0; clippy 0; rustdoc 0; `fuzz` 0;
nextest 1742 passed, 21 failed (the manifest's 18, and the 3 `played_here`, passing once built); conformance 478 passed, 0 failed.
