# 889 — Two merges, and a JBIG2 page drawn from the segments its stream carries whole: rounds 881, 883 and 885 on `main`, and the byte a `/Length` counted is one the standard says is not data

Date: 2026-09-03.
ADR: [0823](../adr/0823-a-jbig2-stream-is-decoded-from-the-segments-it-carries-whole-and-the-byte-a-length-counted-is-not-one.md).
Touched: `crates/pdf-sandbox/src/decode.rs`, `crates/pdf-model/src/image.rs`,
`crates/pdf-model/tests/attachments_filed.rs`, `doc/checks/fixed-documents.toml`,
`doc/conformance/ledger.toml` (§7.4.7, §7.3.8.1), `doc/todo/03-more-corpora.md`,
`doc/todo/38-a-documents-restrictions-have-levels.md`; and the two merge commits before this
round's own.

## The merges

**`round-883` (0b48c06c) is on `main` as `e1785609`, `--no-ff`, and it brings `round-881`
(c6905cb5) with it** — 881 is an ancestor of 883, which merged it before building on it. 881
gives `pdf_syntax::FileBytes` an on-disk form whose window grows until the parse examined
nothing at its end (ADR 0809); 883 sends that open file's descriptor to the confined worker with
`SCM_RIGHTS`, admitting `recvmsg` and `pread64` to the interpreter's allow-list and nothing else,
and makes §12.8.3.2's six digest attempts one pass over `/ByteRange` in 64 KiB windows (ADR
0812). Git found no conflict: the three files both sides had edited — `pdf-syntax/src/document.rs`,
`ledger.toml` and `doc/todo/10` — merged at different lines.

**`round-885` (e8ce96a9) is on `main` as `db6f7a3c`**, `--no-ff`. §7.11.4's embedded file
attached and detached as an edit in `viewer-core`'s log, written by §7.5.6's update at the save,
with all four restriction levels and the messages that answer them (ADR 0814).

**The conflict this round was told to expect did not appear as one, and the two wire extensions
are disjoint by construction rather than by luck.** Both branches extend
`crates/viewer-confined/src/protocol.rs`; 883's `ON_DISK` is 1 in a *new* `open_kind` namespace
beside `BYTES` 0, while 885 takes the next free code in each of the three older ones —
`command_kind::ANSWER` 26 after `VIEW` 25, `encode_edit`'s arms 4 and 5 after `SetFreeText`'s 3,
and `event_kind::ASKING` 16, `WARNED` 17, `ATTACHMENTS_CHANGED` 18 after `SEARCHED` 15. Both
survive; the protocol's round-trip tests and `viewer-ffi`'s four self-checks — the header's
constants against the library's, the entry-point count in two files, the C driver's expected
event-kind count — pass on the merged result without a number moving.

**One conflict the text merge could not see, and it is 881's.** `Document::bytes` returns a
`&FileBytes` rather than a `&[u8]` since the file is read where its offsets point, so 885's two
§7.5.6 prefix assertions in `crates/pdf-model/tests/attachments_filed.rs` stopped compiling.
They take `pdf-syntax`'s own idiom for a document held in memory,
`doc.bytes().whole().expect(…)`, which `write.rs` and `file.rs` already use. And `doc/todo/38`'s
descriptor-route entry said the document's `SCM_RIGHTS` route "was not on `main`" — both are on
`main` now, so it names `write_frame` and says what an attach would need to take it.

The whole `doc/todo/02` §2 sequence ran green on the merged `main` before this round's own
change, and again after it.

## The finding: a stream that was never truncated

`doc/todo/03` §43 left `PDFIUM-1236-1.pdf` named as "a truncated JBIG2 prefix — the same
question ADR 0794 answered for CCITT, one filter over". **It is not truncated, and that is the
finding.** The page is one light grey schematic drawn by a single 3562 × 851 `JBIG2Decode` image
under an `/SMask`; this tree drew it blank on `Xop2: JBIG2: unexpected end of input` while
`poppler` and `mupdf` both draw the diagram and agree on 1.463 levels of ink to a thousandth.
Its 95 declared bytes are a 30-byte page information segment and a 64-byte immediate generic
region — 94 bytes of ISO/IEC 14492 Annex D.3, whole, with the terminating `FF AC` in place. The
ninety-fifth byte is a LINE FEED, and §7.3.8.1 says what it is: "There should be an end-of-line
marker after the data and before endstream ; this marker **shall not** be included in the stream
length." The producer counted it, the codec read it as the start of a segment, and the page went
blank.

**The clause reading, and where it parts from CCITT's.** §7.4.7 states nothing at all about
damaged data — no counterpart to §7.4.6's "[t]he filter shall not perform any error correction
or resynchronization" and none to Table 11's `/DamagedRowsBeforeError` — but it does state the
data's shape: sequential organisation, each segment a header stating its own data length. So
where a stream ends is read off the bytes rather than guessed. And what the rest of the page
shows is stated where §7.4.6 leaves it open: 14492 7.4.8.5's page information segment carries a
default pixel value, "the initial value for every pixel in the page, before any region segments
are decoded or drawn", so a JBIG2 page has no undelivered rows at all and there is nothing to
leave unpainted and no colour for this reader to choose. ADR 0794's answer was the right one for
its clause and is the wrong shape for this one.

ADR 0823: a stream the codec refuses is decoded from the prefix that is whole segments, its
`/JBIG2Globals` alongside it; a lone end-of-line marker past the last whole segment is dropped
in silence, because the standard says it is not data; anything longer is reported beside the
drawing, and where the trimmed stream then fails to *decode* the refusal says both things. The
walk runs only after the codec has refused the pair, which is what makes a short answer safe.

## What it is worth, measured

Over **every document naming `JBIG2Decode` in cleartext under `corpus-cache/`,
`doc/pdf.js/test/pdfs` and `doc/corpora/` — 1523 of them** — surveyed whole before and after
under `tools/bounded.sh --data 8 --tree 12`: **43 incomplete before, 42 after**, with nothing
regressed. `PDFIUM-1236-1.pdf` is complete, reports nothing, and draws at **1.519** where it was
**0**. Of the five documents that reported `unexpected end of input`, only that one was the
marker; the other four carry no whole segment to retry with — two share a stream whose first
segment declares 318 767 114 bytes over 6871 present, and `GHOSTSCRIPT-693285-1.pdf` has a whole
image stream beside a `/JBIG2Globals` truncated inside its one symbol dictionary, which is why
the globals take the same route. Seven further refusals became two-part and more specific.

Two instrument corrections the round made on itself, both by reading rather than by a gate: a
scratch probe was told the wrong stream for `GHOSTSCRIPT-693285-1.pdf`, because its `/Length` is
an indirect reference and a regex read the object number as the length; and a first draft of the
ADR said `find_endstream`'s recovery path hands every filter the marker, which it does not — it
trims one end-of-line sequence, and has since ADR 0366.

## Gates

The full `doc/todo/02` §2 sequence ran twice on `main` under `tools/bounded.sh` (`--data 8`,
`--tree 8` for a build and `12` for a walk, one walk at a time, nothing beside it): once on the
merged tree and once after this round's change, which touches two first-row crates. Every line
exit 0 on both runs. Second run, from the logs: formatting and `clippy` under `-D warnings`
silent for the workspace and for `fuzz/`; **3065 tests passed, 22 skipped**; doctests green; the
corpus gate at **974 documents, 64 incomplete**; the oracle at **1945 pages, 1841 complete, 104
incomplete**; the three text gates green (99.67% of matched words in bounds, 493 of 503
documents fully in); the two censuses green (the drag at 98.91% over 453 documents, 877 of 877
untagged pages answering the empty tree); dates 1514 of 1545; XMP and JPEG 2000 green; quorra at
**958 pages, 929 agree, 22 differ, 7 refused, 16 not comparable**; fixed documents **61 of 61**,
the new row among them; the transform gate at 185.8 pages/s over a floor of 40; the writer over
974 documents, 941 attached and read back, nothing unexplained; conformance green. The first run
differed only in the two figures this round moved: 3058 tests and 60 fixed-document rows.

`tools/worktree.sh close 881 883 885` took the three checkouts and their build directories away
once all three branches were ancestors of `main`. `r867` and `r887` are neighbours' and are
untouched. Not a fifth round by the count, but a merge runs the sequence whole and so does a
change to `pdf-sandbox` and `pdf-model`; §5's binaries were not rebuilt, because no measurement
was taken from them.
