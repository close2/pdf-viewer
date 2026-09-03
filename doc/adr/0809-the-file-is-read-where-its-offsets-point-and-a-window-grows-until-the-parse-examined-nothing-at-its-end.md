# ADR 0809 — The file is read where its offsets point, and a window grows until the parse examined nothing at its end: a 6 GB document opens at the cost of its trailer, its table and page one, the same bytes give the same objects whichever way they are held, and a scan still reads the file whole

Status: accepted. Session 881.
Clauses: ISO 32000-2 §7.5.1, §7.5.2, §7.5.4, §7.5.5, §7.5.8.3 (Table 18), §7.3.8.2, Annex C.4 (informative).
Code: `crates/pdf-syntax/src/file.rs` (`FileBytes::on_disk`, `read`, `whole`, `parse_from`,
`ReadFailure`), `crates/pdf-syntax/src/lexer.rs` (`Lexer::examined`, `note_examined`),
`crates/pdf-syntax/src/parser.rs` (`Parser::examined`, `endstream_examined`),
`crates/pdf-syntax/src/xref.rs` (`read`, `rebuild`, `read_section`, `read_classic_table`,
`header_number_at`, `find_startxref`, `find_trailer_from`), `crates/pdf-syntax/src/document.rs`
(`parse_at`, `with_stated_length`, `whole_for_a_scan`, `scan_refused`),
`crates/pdf-syntax/src/write.rs` (`UpdateError::CannotHold`), `crates/pdf-model/src/signature.rs`
(`signed_bytes` over `FileBytes`), `crates/viewer-core/src/command.rs` (`Command::Open`'s `bytes`),
`crates/viewer-confined/src/protocol.rs` (the `Open` arm), and every host that opens a file itself:
`pdf-viewer`, `pdf-viewer-gtk`, `pdf-viewer-qt`, `pdf-transform`, `pdf-retrieve`, `safedocs`.
Tests: `crates/pdf-syntax/tests/on_disk.rs` (six, and an ignored walk of the pdf.js corpus),
`crates/pdf-syntax/src/file.rs::tests` (two new).
Measured by: `crates/pdf-model/examples/open_cost.rs` (both routes, `OPEN_COST_ROUTE=whole` for
the old one) and `crates/pdf-syntax/examples/callgrind_open.rs` (the same switch).
Opened by ADR 0795's last paragraph and `doc/todo/10`'s row for it.

## Context

`CLAUDE.md` principle 2 states, under *Incremental parsing*: "Opening a document reads the
trailer and the objects page one needs — not the whole file. A 500-page document must open no
slower than a 5-page one." Round 878 (ADR 0795) found the second sentence true of the *parsing*
and the first false of the *bytes*: every host read the file whole into a `Vec<u8>`, `Document`
held it, and `pdf-syntax` read slices of that one buffer, so an honest 6 001 925 614-byte PDF 1.5
file — `batch5/poppler`'s `poppler-44085-1.xz-0.pdf`, 2000 pages, a classic table of 12 070
entries and a ten-digit `startxref` — cost 5.6 GiB of resident memory to show its first page. 878
took the second copy away and left "a file-backed reader" as the next question.

### Measured first

`examples/open_cost` gained a line for the read and a line for the process's peak resident memory
(`VmHWM`), and was run against the tree as 878 left it, three runs each on the two documents this
project quotes startup on and two on the six-gigabyte one, under `tools/bounded.sh --data 8`:

| document | read the file | `Document::open` | page one | peak resident |
|---|---|---|---|---|
| ISO 32000-2, 19.2 MB, 1023 pages | 2.8 ms | 8.1 ms | 5.7 ms | 36.3 MB |
| PDF20_AN001-BPC, 173 KB, 5 pages | 0.09 ms | 0.11–0.15 ms | 1.8 ms | 7.0 MB |
| poppler-44085-1, 6.0 GB, 2000 pages | 1320 ms first run, 424 ms warm | 0.6–0.9 ms | 6.7–8.1 ms | 5 878 MB |

The distance from the principle's sentence is the first and last columns: the trailer, the table
and page one of the 2000-page document cost nine milliseconds, and *reading it* cost half a
second to over a second and 5.6 GiB. The 1023-page document's read is 2.8 ms of a 16.6 ms open —
small, but it is 17% of the open path and the whole of it is bytes page one does not need.

## The three designs

The instruction named three, and the tree's constraints decided between them before the
measurement did.

**(b) A memory map made outside the forbid-unsafe crates and handed in as a slice** is the classic
design and the one poppler, mupdf and pdf.js all use in one form. It is refused on principle 3.
`memmap2::Mmap::map` is `unsafe fn` because the file can change underneath the mapping — a
truncation is a `SIGBUS` no Rust code can catch, and a modification is a `&[u8]` whose contents
change under a borrow, which the compiler assumes cannot happen. That is *untrusted input reaching
unsafe code*, in the exact words of the principle, however far from `pdf-syntax` the `unsafe` block
is placed: the slice that comes out of it is what the parser reads. It costs the sandbox too — the
confined worker has no file system, so the host would map the file and the worker would need a
second mapping of a descriptor handed across, in a crate that touches PDF bytes and therefore
forbids `unsafe`. And it costs the immutability rule its evidence: a `Document` over a mapping is a
function of the bytes *as they were when each page was read*, which is not the premise the oracle
rests on.

**(c) Keeping read-whole** is what 878 chose for its round, with the argument that the process's
limit is the only honest bound on file size. The measurement above says why it is not the answer
to *this* question: the bound decides whether a file opens, and the principle is about what an open
*costs*. A 100 MB scanned book reads in tens of milliseconds warm and hundreds cold, which is the
whole of the launch path's 110 to 119 ms again (`doc/todo/42`), and holds 100 MB for the life of the
window; the six-gigabyte file holds six. Read-whole meets the principle at the sizes the corpus's
median document has and fails it at the sizes that made the owner write the sentence.

**(a) A `FileBytes` backed by an open file, reading on demand what §7.5.4's offsets name** is
what this round built. It costs no dependency, no `unsafe`, and no change to what the confined
worker receives. What it costs is a design inside `pdf-syntax`, which had read slices of one
buffer since its first commit, and that design is the rest of this document.

## Decision

### 1. The file is held one of three ways, and the third is a handle

`FileBytes` gains a variant: `on_disk(path)` opens the file, reads its length from the metadata,
refuses a directory (so a host's sentence about a path it cannot open stays true) and reads
nothing else. It offers `len`, `read(range)` — borrowed in memory, read into a buffer on disk,
clipped to the file as a slice's `get` would be — `whole`, and the window reader below. `Deref`
to `[u8]` is gone, deliberately: a slice of the whole file is exactly the thing a reader of a large
file must not need, and every caller that had one now says which of the three it wants. Positional
reads go through `FileExt::read_at` on Unix, which moves no cursor and so needs no lock, and
through a serialised seek-and-read elsewhere.

### 2. A window grows until the parse examined nothing at its end

Every reader in `pdf-syntax` that starts at an offset — an indirect object, a cross-reference
section, an object's own header, the `endstream` after a stated length — goes through
`FileBytes::parse_from(offset, first, read)`. In memory the closure runs once, over the rest of
the file from the offset, which is the slice `Parser::at` was always given. On disk it runs over a
window of `first` bytes and answers its value beside **how many bytes of the window the value
depended on**; the window is taken only where that count is short of the window's end or the
window reaches the file's end, and otherwise doubled — and grown to at least what the parse said it
needed — and the parse run again.

The count is `Parser::examined`, and it is honest by construction rather than by care. The lexer's
cursor moves backwards only through `seek`, so the furthest position any seek gave up, or the
position now, is the furthest byte a token depended on; one more than that, because a token's end
is decided by the byte after it — `123` followed by the input's end is a shorter number than `123`
followed by `4` — and `<<`, `>>` and a CR LF pair are read one byte past the cursor. The two
look-aheads the lexer does not make are noted by the parser: a stated `/Length` checked for the
`endstream` after it notes the stated end and the keyword's span, whether or not the window reaches
it, so a window shorter than a stream is grown to the stream rather than believed; and a search for
`endstream` that finds nothing notes the whole window. **So a parse taken from a window is the
parse the whole file would have given, because every byte the outcome depended on was the same
byte** — which is the premise the oracle rests on, kept for the on-disk route by the same argument
that keeps `Document` immutable.

The constants are the first read and not the last: an object's window starts at 4 KiB, one page of
memory, and a section's at 64 KiB, about three thousand classic entries. Neither bounds anything —
the growth does, and it is bounded by the file's end and by `try_reserve_exact`, whose refusal is
recorded as a `ReadFailure` and read as the file's end. No hint from the table's own count is
taken, deliberately: a subsection header claiming four billion entries would have the reader ask
for the rest of the file in one window, and doubling costs at most one more parse of the section
than a whole file pays.

### 3. A scan reads the file whole, once, or refuses by name

Where a reader needs every byte — a rebuild of the table from object headers, a `startxref`
further back than the last two kilobytes (ADR 0379's second look), every damaged dictionary, the
writer's appended update, a signature's `/ByteRange` — it asks `FileBytes::whole`, which reads the
file once with the room asked for first (ADR 0795's `hold`) and keeps it, or answers `NoRoom`. So a
damaged file costs on disk what it cost in memory, an intact one costs what it names, and a file
the process cannot hold whole is refused by name at the moment it needs scanning:
`xref::rebuild` answers `NoCrossReferences` with the refusal in its detail, `Document::scan_refused`
records a refusal made after the open — an object the scan would have found is `Object::Null`
there — and `UpdateError::CannotHold` is the writer's. The last of those has no consumer yet and
`doc/todo/10` says so.

### 4. The boundary carries the handle, and the hosts open on disk

`viewer_core::Command::Open`'s `bytes` is a `FileBytes` rather than a `Vec<u8>`. `doc/ui-boundary.md`'s
rule 2 — no file system in the core — holds as it was: the host chose the path and opened it, and
the core reads through what it was handed and never a path of its own. The three windows,
`pdf-transform`, `pdf-retrieve` and the survey open on disk; the confined viewer's host still reads
the file whole, because its worker has no file system and receives the document over a pipe, and
the wire's `Open` arm reads a `FileBytes` whole to send it, refusing as `Uncarried` where it cannot.
The road that closes that is an open descriptor handed to the worker at spawn with `pread64`
allowed through the filter, and it is written in `doc/todo/10` rather than taken here, because it
is a change to the sandbox's shape and this round's question was the reader's.

## What it cost, and what it bought

### Re-measured

The same instrument, the same three documents, the tree with this change, three runs on the small
two and two on the large one; `OPEN_COST_ROUTE=whole` is the old route in the new tree, which is
the route the confined worker still receives a document by.

| document | route | read or open the file | `Document::open` | page one | peak resident |
|---|---|---|---|---|---|
| ISO 32000-2, 1023 pages | whole | 3.2–3.6 ms | 5.8–8.0 ms | 6.1–7.3 ms | 36.5 MB |
| | **on disk** | 0.01–0.07 ms | 5.9–7.5 ms | 6.5–7.3 ms | **17.7 MB** |
| PDF20_AN001-BPC, 5 pages | whole | 0.09–0.11 ms | 0.12–0.13 ms | 1.5–1.9 ms | 6.9–7.1 MB |
| | **on disk** | 0.01–0.04 ms | 0.11–0.20 ms | 1.3–1.5 ms | 6.8–7.0 MB |
| poppler-44085-1, 2000 pages, 6.0 GB | whole | 554–1615 ms | 0.8–1.2 ms | 8.6–8.9 ms | 5 878 MB |
| | **on disk** | 0.02–0.07 ms | 1.3–4.2 ms | 15.8–18.6 ms | **17.1 MB** |

So the principle's sentence is met in both halves: the 2000-page document opens and draws its
first page in about 20 ms, against the 5-page document's 1.5 ms and the 1023-page one's 13 ms,
and holds 17 MB where it held 5.6 GiB. The 1023-page document's open path lost its 3 ms read and
half its peak. The 5-page document is unchanged within the noise of a run, which is the number the
design was most likely to lose: a small file read whole is one `read` and a small file on disk is
one `pread` per object, and page one's objects are few.

The page-one line for the six-gigabyte document is the one honest cost in the table: 7 to 10 ms
more on disk, which is its one image stream read through a window that is grown once to the
stated `/Length` and then copied out of the window into the stream's own `Arc<[u8]>` — two reads
and one copy of the stream, where the whole route paid one memcpy out of a buffer the kernel had
already prefetched. It is paid once, for a page that draws in tens of milliseconds, in place of a
second of reading; it is not paid at all for a page whose streams are small.

**Under callgrind**, which counts instructions and is therefore deterministic: `examples/callgrind_open`
opening ISO 32000-2 ten times, trailer and table only — **610 698 555 instructions whole against
611 461 037 on disk**, 0.12% more, which is the window reads and the one re-parse of the section
whose 64 KiB window the 101 318-entry table runs past. The whole route's number does not count
its own read, which is a kernel memcpy of 19 MB that no instruction of this program performs.

**And the viewer itself**, `target/pdf-viewer --trace` under `Xvfb` with `lavapipe`, read off
`/proc/<pid>/status` twelve seconds in: the six-gigabyte document *opened on its own thread* at
78 ms after launch and the window peaked at **281 MB** resident; ISO 32000-2 opened at 81 ms and
peaked at 299 MB. Before this round the first of those was not a number the window could reach
under an 8 GiB limit without the file's whole length beneath it.

### What it cost

- `pdf-syntax` gained a second way to hold a file, and every reader in it that starts at an
  offset now says how far it looked. The count is one `max` in `Lexer::seek`, one field, and two
  notes in the parser's stream delimiting; the section reader re-parses a section whose table runs
  past 64 KiB, at most once more than a whole file pays.
- `FileBytes` lost `Deref<[u8]>`, and the seven consumers of the whole slice — the signature
  ranges, the writer, the header version, the notes' length, the font cache's identity, the
  census, the tests — each say which of `read`, `whole` or `len` they meant.
- `Command::Open`'s `bytes` changed type, which was 132 `.into()`s in tests and examples and one
  `whole()` in the confined wire.
- One open file descriptor per open document. A file that shrinks under the reader reads as though
  it ended where it now does, one that grows is read to its opened length, and a read that fails is
  remembered as a `ReadFailure` and read as the end — none of the three is a panic, and all three
  are outside the contract that a document is a function of its bytes.
- A dependency: none. `unsafe`: none. A number of this program's own: two window sizes, each the
  first read and not a bound, argued above.


## Consequences

- A document opens at the cost of its trailer, its table and page one's objects, whichever its
  size; the file's length is a number in the metadata and not a resident cost.
- The same bytes give the same objects whichever way they are held — pinned by
  `tests/on_disk.rs` over every document in `doc/` on every run, and over the pdf.js corpus by
  hand (964 documents agreeing on 112 574 objects, 10 refused both ways the same). The oracle's
  verdicts, taken in memory, therefore bind the route the viewer runs.
- A damaged file costs what it did; a file too large for the process to scan is refused by name
  rather than read a window at a time twice over.
- `FileBytes` has no `Deref`; a caller that wants the whole file says so and handles `NoRoom`.
- One open file descriptor per open document, for its life.
- Deliberately not done, each with its road written in `doc/todo/10`: the confined path, a
  streaming digest for signatures, a consumer for `Document::scan_refused`.
