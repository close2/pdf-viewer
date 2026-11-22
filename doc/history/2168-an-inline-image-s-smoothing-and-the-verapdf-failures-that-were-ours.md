# 2168 — An inline image's smoothing, and the veraPDF failures that were ours

Session 2168, an archive round in the `lane3` worktree: the owner's A978, then round 2164's veraPDF
failures that are this tree's, by outputs. ADRs 3373 to 3376; no question. Ledger rows 8.9.7, 9.9,
9.8.1 and 12.5.5 (`implemented`, `implemented`, `implemented`, `departed`, before and after).

**A978 (ADR 3373).** `as-shown` writes an inline image's `/I true` false inside its `BI` dictionary
at the loss `image-smoothing`, R02's history entry naming the streams. `TAMReview` and `bug1065245`
convert at 2b and 4 and pass veraPDF. Every byte outside a `BI`…`ID` span is the producer's: the
module's tests, and qpdf's decoded streams of the four outputs (2 and 3 edits, each the value
token). Renders: `bug1065245` identical; `TAMReview` differs in a 127×8 strip, its 1×8 inline
image stretched without smoothing — the loss as named.

**Fonts (ADR 3375).** A `/FontFile` program framed in PFB headers, with a hexadecimal encrypted
portion (§9.9 NOTE 2), or with lengths that do not divide it is written unframed, binary, measured.
Our validator missed it: a PFB program named no format, so the row skipped it; lengths were read
nowhere. `bug860632`, `issue4630` pass veraPDF; `issue14462_reduced` still fails inside its Private
dictionary (the program's content).

**Validator readings (ADR 3374).** An empty `/CharSet ()` lists no name (`issue2462`, now removed
and passing veraPDF); a button state naming the name `null` is no appearance (`issue15557`, now
refused instead of written).

**Diagnosed, left (ADR 3376).** `475121`: the lexer reads a 34-digit integer as a real.
`PDFBOX-4532-reduced`: the row finds the five properties, and the reach exemption withdraws them
under an orphan object. `issue1350`: no row reads `/BaseFont`'s type.

**Census.** With the pack, pdf.js `as-if-printed` 942 at 2b (was 943) and 956 at 4 (957), the
one being `issue15557`, refused rather than written wrong; `as-shown` 908 (907) and 921 (920);
corpora unchanged. Without the pack one worse at each `as-if-printed` part (`issue15053`), as before.
veraPDF over the kept outputs: 95 failing outputs of 29 documents over 24 rules (was 109, 33, 26),
20 unfinished. A first census caught a stricter 6.3.3 reading and a Type 1 length check refusing
`checkbox-bad-appearance` and `bug1157493`, which veraPDF passes; both readings were narrowed
(ADRs 3374, 3375), the census re-taken, and the two documents' outputs pass veraPDF.

**Gates.** Over `-p pdf-transform -p pdf-archive -p viewer-qt`: fmt 0; clippy 0; `pdf-sandbox` build
0; nextest 1706 passed, 0 failed, the empty manifest's; rustdoc 0; `fuzz` check 0 (`Rewrite` and
`pdf_archive` gained items); conformance 0 after a
quotation was set to §8.9.5.3's own words and an ADR's scratch path replaced by its figures.
