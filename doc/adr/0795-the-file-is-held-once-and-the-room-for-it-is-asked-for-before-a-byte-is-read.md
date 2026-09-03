# ADR 0795 — The file is held once, and the room for it is asked for before a byte is read: a 6 GB PDF opens at the cost of one copy, the process's own limit is the bound and it refuses by name, and a predictor's row buffers are sized to the data rather than to `/Columns`

Status: accepted. Session 878.
Clauses: ISO 32000-2 §7.5.1, §7.5.4, §7.5.5, §7.5.8.3 (Table 18), Annex C.4 (informative), §7.4.4.4.
Code: `crates/pdf-syntax/src/file.rs` (`FileBytes`, `read_file`, `NoRoom`),
`crates/pdf-syntax/src/document.rs` (`Document::bytes`, the `open` family),
`crates/pdf-syntax/src/filter.rs` (`apply_predictor`), `crates/pdf-model/src/content/font.rs`
(`Kept::bind`), and every host that reads a document from disk: `viewer-ui`'s three binaries,
`viewer-gtk`, `viewer-qt`, `pdf-transform`, `pdf-retrieve`, `safedocs`.
Tests: `crates/pdf-syntax/src/file.rs::tests` (five), in particular
`a_document_holds_the_vector_it_was_opened_from` and
`a_length_the_process_cannot_hold_is_refused_by_name`;
`crates/pdf-syntax/src/filter.rs::tests::a_stated_row_wider_than_the_data_costs_the_data_and_unfilters_the_same`;
`crates/pdf-model/tests/hostile_budgets.rs::an_image_stating_a_predictor_row_wider_than_its_data_still_draws`.
Opened by `doc/todo/03` section 40, which left `batch5/poppler` unsurveyed behind one allocation.

## Context

Round 876 walked `batch5/poppler` twice and surveyed it neither time: at twelve rayon threads and
at four, one document asked the allocator for **6 001 925 632 bytes in one allocation** and the
survey aborted under `tools/bounded.sh --data 8`, taking the directory's 1586 verdicts with it.
Which document was not chased, and `doc/todo/10` was named as the file the question belongs to
before this one: a six-gigabyte allocation that passes every budget this tree has is a bound
missing, not a document.

This round walked the directory one document per process under `--data 2 --tree 4`, four lanes
side by side, and **every one of the 1586 exited 0.** That is the first finding, and it is the
reason the allocation had escaped every budget: the failing allocation is not a decode, a raster or
a table sized from a stated dimension — it is the *file*. `poppler-44085-1.xz-0.pdf` is
**6 001 925 614 bytes**, a `%PDF-1.5` file filed against poppler in December 2011 whose
`startxref` is the ten-digit `6001684110`. It is not hostile; it is a large PDF, exactly the kind
§C.4 anticipates:

> A PDF cross-reference table (see 7.5.4, "Cross-reference table") allocates ten digits to
> represent byte offsets, which limits the size of a PDF file to 10 10 bytes (approximately 10
> gigabytes). However crossreference streams (see 7.5.8, "Cross-reference streams") allow PDF
> files to be even larger.

(§C.4; the `10 10` is the Markdown conversion's rendering of 10¹⁰.)

And the allocation's size says which allocation it is: 6 001 925 614 plus an `Arc`'s sixteen-byte
header, rounded up to eight, is 6 001 925 632. **It is `Arc<[u8]>: From<Vec<u8>>`** — the copy
`Document::open` made of the vector every host handed it, because an `Arc<[u8]>` needs its
strong and weak counts *in front of* the bytes and a `Vec`'s buffer has no room for them. So the
survey read the file (5.59 GiB, resident), then asked for a second 5.59 GiB to copy it into, and
under an 8 GiB `RLIMIT_DATA` the second request was the one that failed. It failed as an abort,
because that `From` impl allocates infallibly.

Under `--data 2` the same document reported cleanly — `unusable: … out of memory` — for a reason
worth writing down: `std::fs::read` reserves the file's length with `try_reserve_exact` before it
reads, so the *first* 5.59 GiB was already a typed refusal, `io::ErrorKind::OutOfMemory`. The path
into `Document::open` had one fallible allocation and one infallible one of the same size, and the
infallible one was the copy nobody needed.

## Decision

### 1. The bytes a host read are the bytes the document holds

`pdf_syntax::Document` no longer stores an `Arc<[u8]>`. It stores a `FileBytes`, a small enum
behind an `Arc` that holds either the `Vec<u8>` a host handed over — *taken as it is*, one
sixteen-byte allocation for the `Arc` and not one byte copied — or a slice that was already
reference-counted when it arrived. The `open` family takes `impl Into<FileBytes>`, with `From`
impls for `Vec<u8>`, `Arc<[u8]>`, `Box<[u8]>`, `[u8; N]`, `&[u8; N]`, `&Vec<u8>` and `&[u8]`;
only the last two copy, because a borrowed slice has to be owned to be held, and both are test
shapes. `Document::bytes` returns `&FileBytes`, which dereferences to `&[u8]`, so every caller that
wanted the slice — the signature checks, the writer, the header-version reader — compiles
unchanged. The one caller that wanted the *identity* of the bytes, `pdf-model`'s font cache, which
binds itself to a document by address and holds the allocation so the address cannot be reused
(ADR 0317's invariant), asks `FileBytes::same` and holds a clone.

Measured on the document this is about: the survey opens and draws page one of
`poppler-44085-1.xz-0.pdf` in **3.2 s at a 5.58 GiB peak** under `--data 8`, `complete`, where it
aborted at 11.2 GiB asked. The peak is the file, once.

What it costs: one more pointer indirection on each dereference of the document's bytes — an
`Arc<Vec<u8>>` is a pointer to a pointer where an `Arc<[u8]>` was a pointer and a length. Lexing
takes the slice once per object load and works on it, so the difference is one load per object
rather than one per byte. Measured under callgrind, which counts instructions and is deterministic
(`examples/callgrind_open`, ISO 32000-2, 101 318 objects): **611 473 529 instructions before and
610 350 909 after**, 0.18% *fewer*, because the copy of the 22 MB file that every open paid is
gone and the extra load per object does not show.

### 2. The room is asked for before the first byte is read, and the refusal has a name

`pdf_syntax::read_file(path)` replaces `std::fs::read(path)` in every host that opens a document:
`pdf-viewer`, `pdf-viewer-confined` (all three of its reads), the GTK and Qt hosts, `pdf-transform`
(both verbs that take a file), `pdf-retrieve` and the survey. It reads the file's length from its
metadata, asks for exactly that much with `try_reserve_exact`, and only then reads. A length the
process cannot hold comes back as `io::ErrorKind::OutOfMemory` carrying `NoRoom { length }`, whose
message is *the file is N bytes and this process cannot hold it* — the length by name, typed so a
host can read it back, under the `io::Error` every one of those hosts already handles. Each host's
own sentence is unchanged: *cannot read <path>: the file is 6001925614 bytes and this process
cannot hold it*.

**There is deliberately no number.** The instruction that opened this round asked for a budget
whose number is justified here, and the justification is that the only honest number is the
process's own: `doc/todo/10`'s brief is the owner's — "why should we prevent people from using our
viewer for very complicated PDFs" — and a 5.6 GB PDF is such a document on a machine that holds it.
A constant of this program's would refuse it on a 64 GB workstation to protect a phone, and
`doc/todo/10` §6's first rule is that "[n]othing arbitrary may be replaced by something equally
arbitrary". What binds instead is whatever limit the process runs under — `RLIMIT_DATA`, a cgroup,
the confined worker's `INTERPRETER_ADDRESS_SPACE_LIMIT` — asked *once*, before a byte is read, and
answered as a typed error rather than an abort. That is the same shape ADR 0597 gave the confined
worker's frames: check, then `try_reserve`, then read. The value that drives it is the file's
length, which is what §7.5.1's four parts are all addressed by, and §7.5.4 and Table 18 are the
clauses that make that a requirement on a reader rather than a convenience.

What this does not do, and is written down so nobody reads it as done: it does not stop reading
the whole file. `CLAUDE.md`'s "[o]pening a document reads the trailer and the objects page one
needs — not the whole file" is still untrue of the *bytes*, which are read whole and held whole;
it is true of the *parsing*. A file-backed reader that seeks to what §7.5.4's offsets name is the
road that would make a 6 GB open cost its trailer rather than 5.6 GiB of resident memory, and it is
a design of its own — the whole of `pdf-syntax` reads slices of one buffer — which `doc/todo/10`
now carries as the next question for this file size.

### 3. The class, checked, and one more member closed

The round was asked to look for the same class across `doc/todo/10`'s bounds — *a size computed
from stated dimensions before any byte is read* — and to close the class where that is cheap.
Every allocation site in `pdf-syntax`, `pdf-model`, `pdf-font`, `pdf-render`, `render-cpu` and
`pdf-sandbox` whose size is an expression rather than a literal was read (135 of them, by grep for
`with_capacity`, `vec![…; n]` and `resize`). Three findings:

- **Guarded already, by a number with an argument**: every image path is behind
  `image::MAX_SAMPLES` or the sandbox's `MAX_PIXELS`, including the combined grid of an image and
  its mask (`image.rs`'s `grid > MAX_SAMPLES.max(image)`); sampled functions behind
  `function::MAX_SAMPLES`; ICC tables behind `MAX_CLUT`, a tag count of 1024 and a curve length of
  2^17; the structure tree's table grid behind `MAX_TABLE_COLUMNS` and `MAX_TABLE_GRID`; mesh
  tessellation by a constant; the worker's response by `MAX_RESPONSE`.
- **Not this program's input**: the sandbox worker's request frame sizes `vec![0u8; primary_len]`
  from a header its *parent* wrote, and the worker runs under `RLIMIT_AS`.
- **One member open, and closed here**: §7.4.4.4's predictor. `apply_predictor` sized its two row
  buffers to `row_len` — `/Colors × /BitsPerComponent × /Columns`, every factor the file's — before
  reading a byte, so a six-byte stream stating `/Columns 1099511627776` asked for two terabytes and
  aborted. A row copies at most `min(row.len(), row_len)` bytes and a row is a chunk of the data,
  so `min(row_len, data.len())` holds every row the loop will ever fill and the output is
  byte-identical. No behaviour changes, no report changes; only the allocation is bounded by the
  data it unfilters. The unit test and the hostile fixture both abort against the old sizing —
  `memory allocation of 1099511627776 bytes failed` — which is trap 13's control, run.

## Consequences

- `Document::open` costs the file once. A host that holds a 5.6 GB PDF holds 5.6 GiB, and the
  survey that could not walk `batch5/poppler` walked it (`doc/todo/03` §41).
- A file the process cannot hold is refused by name before it is read, typed as
  `io::ErrorKind::OutOfMemory` with `NoRoom` inside, and no host had to change a signature.
- A predictor's `/Columns` can no longer command an allocation the stream does not fill.
- `pdf_syntax::Document::bytes` returns `&FileBytes` rather than `&Arc<[u8]>`; a caller that needs
  an `Arc<[u8]>` of the document does not exist in this tree, and one that needs identity asks
  `same`.
- Deliberately not done: a file-backed, seeking reader; a number of this program's own for file
  size. Both are recorded as decisions rather than left as omissions.
