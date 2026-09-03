# 881 — The file is read where its offsets point, and a window grows until the parse examined nothing at its end: `batch5/poppler`'s six-gigabyte document opens at the cost of its trailer, its table and page one, the same bytes give the same objects whichever way they are held, and a scan still reads the file whole

Date: 2026-09-03.
ADR: [0809](../adr/0809-the-file-is-read-where-its-offsets-point-and-a-window-grows-until-the-parse-examined-nothing-at-its-end.md).
Touched: `crates/pdf-syntax/src/file.rs`, `lexer.rs`, `parser.rs`, `xref.rs`, `document.rs`,
`version.rs`, `write.rs`; `crates/pdf-syntax/tests/on_disk.rs` (new);
`crates/pdf-syntax/examples/callgrind_open.rs`, `crates/pdf-model/examples/open_cost.rs`;
`crates/pdf-model/src/signature.rs` and its census example; `crates/viewer-core/src/command.rs`,
`open.rs`, `viewer.rs`; `crates/viewer-confined/src/protocol.rs`; `crates/pdf-transform/src/lib.rs`
and its binary; the document-opening sites of `viewer-ui`'s two binaries, `viewer-gtk`,
`viewer-qt`, `tools/pdf-retrieve`, `tools/safedocs`; every `Command::Open` construction in tests
and examples; `doc/conformance/ledger.toml` (§7.5.1, §7.5.4, §7.5.5, §7.5.8.3, §7.3.8.2),
`doc/todo/10-bounds-that-cap-size.md`, `doc/todo/42-the-launch-path.md`, `doc/ui-boundary.md`,
`doc/state-of-play.md`.

## Measured first

`examples/open_cost` gained a line for reading the file and a line for the process's peak resident
memory, and was run against the tree as round 878 left it. ISO 32000-2 (19.2 MB, 1023 pages): 2.8 ms
to read, 8.1 ms to open, 5.7 ms for page one, 36 MB peak. The five-page application note: 0.09 ms,
0.11 ms, 1.8 ms, 7 MB. `poppler-44085-1.xz-0.pdf` (6.0 GB, 2000 pages, a classic table of 12 070
entries): **424 to 1320 ms to read, 0.6 ms to open, 7 ms for page one, 5 878 MB peak.** The
principle's first sentence — the trailer and the objects page one needs, not the whole file — was
true of the parsing and false of the bytes, and the distance was the whole of the read column.

## The design, and the two refused

ADR 0809 weighs the instruction's three. A memory map made outside the forbid-unsafe crates is
refused on principle 3 — a truncation under the map is a `SIGBUS` nothing catches, a modification
is a `&[u8]` changing under a borrow, and the slice that comes out of the `unsafe` block is what
the parser reads, however far away the block is — and it would cost the confined worker a second
mapping in a crate that forbids `unsafe`. Keeping read-whole answers whether a file *opens* and not
what an open *costs*; it meets the principle at the corpus's median size and fails it at the sizes
the sentence was written for. What was built is the third: `FileBytes::on_disk` keeps the file
open, and every reader in `pdf-syntax` that starts at an offset goes through
`FileBytes::parse_from`, which in memory runs once over the rest of the file and on disk runs over
a window that is grown — doubled, and to at least what the parse said it needed — until the parse
examined nothing at the window's end. `Parser::examined` is that count: the lexer's cursor moves
backwards only through `seek`, so the furthest position a seek gave up, plus one for the byte a
token's end is decided by, plus the two look-aheads a stream's delimiting makes. A scan reads the
file whole through `FileBytes::whole`, once, or is refused by name — `xref::rebuild`'s
`NoCrossReferences`, `Document::scan_refused`, `UpdateError::CannotHold`. `Command::Open` carries
a `FileBytes`; the three windows, `pdf-transform`, `pdf-retrieve` and the survey open on disk; the
confined host still reads whole, because its worker has no file system, and the wire's `Open` arm
reads a `FileBytes` whole to send it.

`tests/on_disk.rs` opens every document in `doc/` both ways and compares every object the table
names, and its ignored walk did the same over the pdf.js corpus: **964 documents agreeing on 112 574
objects, 10 refused both ways the same.** Four hostile fixtures beside it — an offset past the end,
a `/Length` that runs off the file, a stream longer than a window, an object longer than a window —
and a `/Prev` loop across two sections; run against a reader that never grows its window (trap
13), four of the six fail.

## Re-measured

The same instrument, the same documents. ISO 32000-2 on disk: 0.01–0.07 ms to open the file,
5.9–7.5 ms to open, 6.5–7.3 ms for page one, **17.7 MB** peak against 36.5. The five-page note:
unchanged within a run's noise. The six-gigabyte document: **0.02–0.07 ms, 1.3–4.2 ms, 15.8–18.6 ms,
17.1 MB** against 554–1615 ms, 0.8–1.2 ms, 8.6–8.9 ms and 5 878 MB — the page-one line is the one
honest cost, one image stream read through a window grown once to its `/Length`. Under callgrind,
ten opens of ISO 32000-2: 610 698 555 instructions whole, 611 461 037 on disk, 0.12% more, and the
whole route's read is a kernel memcpy no instruction counts. `target/pdf-viewer --trace` under
`Xvfb`: the six-gigabyte document opened on its own thread 78 ms after launch and the window peaked
at 281 MB resident; ISO 32000-2 at 81 ms and 299 MB.

## Gates

The whole `doc/todo/02` §2 sequence under `tools/bounded.sh` (`--data 8`, `--tree 8` for a build
and `12` for a walk), every line exit 0 on the first pass: formatting and `clippy` under
`-D warnings` silent for the workspace and for `fuzz/`; **3009 tests passed, 21 skipped**; doctests
green; the corpus gate at **974 documents, 64 incomplete**; the oracle at **1945 pages, 1841
complete, 104 incomplete**; the three text gates green (99.67% of matched words in bounds); the two
censuses green (the drag at 98.91% over 453 documents); dates 1514 of 1545; XMP and JPEG 2000
green; quorra at **958 pages, 932 agree, 22 differ, 4 refused**; fixed documents **58 of 58**; the
transform gate at 216.7 pages/s over a floor of 40; the writer over 974 documents, 941 attached and
read back, nothing unexplained; conformance green. Every figure the same as round 878's, which is
what a change to how the bytes are held and not to what they mean should show. Not a fifth round,
but §5's binaries were rebuilt and installed because a measurement was taken from them.
