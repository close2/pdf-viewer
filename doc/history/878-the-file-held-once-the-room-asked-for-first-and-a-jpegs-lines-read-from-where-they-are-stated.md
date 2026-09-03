# 878 — The file held once, the room asked for first, and a JPEG's lines read from where they are stated: `batch5/poppler`'s six-gigabyte document opens at the cost of one copy, a predictor's row buffers are sized to the data, the tracker is walked, and a `DNL` marker defines a frame's number of lines

Date: 2026-09-03.
ADRs: [0795](../adr/0795-the-file-is-held-once-and-the-room-for-it-is-asked-for-before-a-byte-is-read.md),
[0799](../adr/0799-a-jpegs-number-of-lines-is-read-from-both-places-the-encoded-data-states-it.md).
Touched: `crates/pdf-syntax/src/file.rs` (new), `crates/pdf-syntax/src/document.rs`,
`crates/pdf-syntax/src/filter.rs`, `crates/pdf-syntax/src/lib.rs`, `crates/pdf-model/src/image.rs`,
`crates/pdf-model/src/content/font.rs`, `crates/pdf-model/examples/signature_algorithm_census.rs`,
`crates/pdf-model/tests/dct_components.rs`, `crates/pdf-model/tests/hostile_budgets.rs`, the
document-reading sites of `crates/viewer-ui`'s three binaries, `crates/viewer-gtk`, `crates/viewer-qt`,
`crates/pdf-transform`, `tools/pdf-retrieve` and `tools/safedocs`; `doc/checks/fixed-documents.toml`,
`doc/conformance/ledger.toml`, `doc/todo/03-more-corpora.md`, `doc/todo/10-bounds-that-cap-size.md`.

## The allocation

`doc/todo/03` §40 left `batch5/poppler` unsurveyed behind one allocation of 6 001 925 632 bytes.
The directory was walked one document per process under `tools/bounded.sh --data 2 --tree 4`, four
lanes side by side each waiting on any neighbour's walk, and every one of the 1586 exited 0 — which
was the finding. `poppler-44085-1.xz-0.pdf` is 6 001 925 614 bytes, `%PDF-1.5`, `startxref
6001684110`; add an `Arc`'s sixteen-byte header and round to eight and the number is the survey's.
Under `--data 2` it had reported `out of memory` cleanly, because `std::fs::read` reserves with
`try_reserve_exact`; under `--data 8` the read succeeded and `Arc<[u8]>: From<Vec<u8>>`'s copy did
not. Round 876's own log in the scratchpad confirmed the sentence and the four-thread run.

ADR 0795 has the decision: `pdf_syntax::FileBytes` (a `Vec` taken as given or an `Arc<[u8]>`
shared, behind one enum), `Document::open` over `impl Into<FileBytes>`, `bytes()` returning
`&FileBytes`, the font cache binding by `FileBytes::same`; and `pdf_syntax::read_file`, which asks
`try_reserve_exact` for the file's whole length before the first byte and answers `NoRoom { length }`
under `io::ErrorKind::OutOfMemory`. No number of this program's own, on `doc/todo/10`'s brief; the
ADR argues why the instruction's "budget's number" is the process's limit and not a constant. The
document opens and draws page one `complete` in 3.2 s at a 5.58 GiB peak under `--data 8`. A
callgrind A/B of `examples/callgrind_open` over ISO 32000-2, the pdf-syntax patch reversed with
`git apply -R` and reapplied: 611 473 529 instructions before, 610 350 909 after.

The class — a size computed from a stated dimension before any byte is read — was read across 135
allocation sites in `pdf-syntax`, `pdf-model`, `pdf-font`, `pdf-render`, `render-cpu` and
`pdf-sandbox`. All but one are behind a bound with an argument; the one is §7.4.4.4's predictor,
whose two row buffers were `vec![0u8; row_len]` from `/Columns × /Colors × /BitsPerComponent`. They
are `min(row_len, data.len())` now, output byte-identical, and both tests were run against the old
sizing first (trap 13): `memory allocation of 1099511627776 bytes failed`, `SIGABRT`, twice.

## The walk, and the head

`batch5/poppler` surveyed whole at twelve rayon threads under `--data 8 --tree 12`, 19.2 s, 3.59 GiB
peak, the six-gigabyte document surveyed on its own beside it: 1586 documents, 6 unopenable, 3
locked, 2 encrypted beyond us, 27 pageless, 183 incomplete, 0 slow. `doc/todo/03` §41 has the
reports by kind, the pageless read against `pdfinfo` and `mutool` (eighteen and four of them count a
page; every one is a hand-mangled tree of §34's population), and the ranking — ours flattened on
white against `pdftoppm -cropbox` and `mutool draw` at 72 dpi over all 183, round 876's script with
its two traps already sprung.

The head was at the dark end with both references agreeing to a third of a level:
`poppler-61994-0.pdf`, ours 60.4 against `poppler` 5.38 and `mupdf` 5.03. Looked at, the page was a
scanned letter squeezed into the top five per cent of a grey sheet. A marker walk: `SOF0` `Y =
65535`, a `DRI`, one scan, and after its data a `DNL` marker stating 3486 lines, thirteen more than
the dictionary's 3473. ISO/IEC 10918-1 section B.2.5 makes a `DNL` at the end of the first scan
define or redefine `Y`; §7.4.8 puts the dimensions in the encoded data; `zune-jpeg` reads the
header alone, refuses a `DNL` it meets and pads the header's grid. ADR 0799: `image::frame_as_defined`
walks the markers, writes the `DNL`'s count into `Y` and takes the six-byte segment out before the
decoder sees the bytes. The letter draws at 5.24 by the ranking's instrument; the fixed-documents
gate reads 10.478 on its own and that is the band. The test over `dct_components.rs`'s hand-written
frame pins `Y = 65535`, `Y = 0` and `Y = 8` beside a `DNL` of 8, and against the tree without the
fix it reported the decoder's refusal and an 8 × 65535 frame.

## Gates

The whole `doc/todo/02` §2 sequence under `tools/bounded.sh` (`--data 8`, `--tree 8` for a build
and `12` for a walk, each walk waiting on any neighbour's). Two failures of this round's own on the
first pass, both fixed and the lines re-run: `clippy` under `-D warnings` on a `format_push_string`
in a new test, and the citation checker on `§B.2.5` after ISO/IEC 10918-1 in three files and a
ledger row, which wants "section". Final figures: formatting and `clippy` silent for the workspace
and for `fuzz/`; **2989 tests passed, 20 skipped**; doctests green; the corpus gate at **974
documents, 64 incomplete**; the oracle at **1945 pages, 1841 complete, 104 incomplete**; the three
text gates green (99.67% of matched words in bounds, 493 of 503 documents fully in); the two
censuses green; dates 1514 of 1545; XMP and JPEG 2000 green; quorra at **958 pages, 932 agree, 22
differ, 4 refused**; fixed documents **58 of 58**, this round's row among them; the transform gate
at 193.8 pages/s over a floor of 40; the writer over 974 documents, 941 attached and read back,
nothing unexplained; conformance green. Not a fifth round, so §5's binaries were not rebuilt.
