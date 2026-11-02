# 2640 — A content-stream rewrite leaves what a photograph took off, and a shared program is restated in a copy

Session 1903, slot 3 of batch 131, an "archive" round.
Status: implemented in `crates/pdf-transform/src/archive/{hexadecimal,languages,prepare,decision,fonts,rewrite,settle}.rs`,
with fixtures in `crates/pdf-transform/tests/archive.rs`, `tests/archive/photographed.rs` and `settle.rs`'s own tests.
ADR 2641 is unused.
Context: ADR 0947 (nothing changes that no failed requirement asked for), ADR 0988 (a rewrite is proved), ADR 2516
(a page photographed answers a fault of its content stream), ADR 2066 and 2067 (the `/Lang` respelling carries the
digit repair).

## 1. A stream a photograph took off is not a content stream of the output

A producer's update reused object 6 as the catalog's `/Metadata` while page 3 still named `/Contents 6 0 R`
(govdocs1 `064034`). Read as a content stream the packet is undefined operators and hexadecimal strings of every
length, so page 3 was photographed; but §7.3.4.3's repair (`hexadecimal::complete`) had already written a `0` before
29 of the packet's `>`, and the stream rewriter returns that repair before the packet writers, so the identification
edit was lost too. The output failed three metadata rows the source met.

The decision: once the photographs are known, `hexadecimal::taken_off_by` names every stream that photographed pages
alone name as `/Contents` and that the survey did not open as a resource, and both content-stream rewrites —
`Completed::leaving_out` and `languages::Restated::leaving_out` — drop those streams and their counts. A page the
walk cannot name by object counts as unphotographed, so a stream it draws is kept. Where nothing is left for the
digit repair, the row `file-structure/hexadecimal-string-digits` is answered by the photograph as the fenced rows are
(`decide`), rather than reported as a repair done zero times.

**Trap.** An object number is not a role: one stream can be a page's contents and the catalog's metadata at once,
and a rewrite keyed by object id that decides by one role writes over the other.

## 2. A program another font shares is restated in a copy where in place would break it

In `507618` fonts F8 and F1 share descriptor 96 and its `/FontFile3` 115. F8's `/Widths` disagree with the program,
F1's agree; restating 115 in place made F1 disagree. `restate_metrics` now asks, before writing in place, whether any
other surveyed font drawing from the same program states for a glyph it shows an advance the restatement contradicts
(a font it cannot read counts as one that does), and whether the program was already restated for another font. If
either, the restated program goes into a fresh object with a copy of this font's descriptor naming it, and the
dictionary naming the descriptor (the font's, or a `Type0`'s descendant) is pointed at the copy; the restatement is
counted at that redirect. Both objects are the file's own with the advances restated: no new mark. Where the holder is
direct, the descriptor names the program under neither `/FontFile2` nor `/FontFile3`, or no number is free, the
conversion is refused by `SHARED_PROGRAM_NOT_COPIED`. The second check closes the silent overwrite of `metrics.at` by
two fonts restating one program.

## 3. A statement the second pass composes is the report's statement too

`settle::fold` joined the second conversion's photographs to the report and dropped its `statement` and `unstated`.
The premise was that the first pass's page then omits the second's photograph; it does not hold as stated: the
second conversion runs the first's plan, so a loss it takes under a row naming `append-statement` is stated on a page
*it* appends, after the first's statement pages, which its source already ends in. What was lost was the report's
record of that page, and with it every sentence that names where a loss is stated. `fold_statement` now joins the
two: the sites of both, the pages of both (one run, the first's then the second's), the objects counted together,
the face the first's (the second sets its text in the document's own face only because the first embedded the face
it names); a first pass that stated nothing takes the second's record whole; `unstated` is extended the same way.
Rewriting the first page to carry the second's entries would need the first source's objects inside the second pass,
which a pass over the first output does not have. No corpus document reaches a second-pass photograph: the
construction refuses an Identity-H face (the only one the text-as-image gate photographs for) and a symbolic face's
appearance left the second pass refused, so the fixture is the fold's own unit test.

Against HEAD's census, with and without the CJK pack, `as-if-printed` converts 424 corpora documents at 2b (from 422)
and 432 at 4 (from 430), and refuses 42 at both parts (from 44): `064034` and `507618`, whose six `output-fails`
lines are the only lines that moved.

## 4. Cost

Two objects per copied font and one page-tree walk when a page is photographed; none otherwise.
