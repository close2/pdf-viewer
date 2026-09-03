# ADR 0812 — The document crosses the confinement as a descriptor, and a signature is digested a window at a time: the confined viewer holds no byte of a 6 GB file, the worker reads it through two system calls and can still name no path, and a scan the process cannot hold is said out loud

Status: accepted. Session 883.
Clauses: ISO 32000-2 §7.5.1, §7.5.4, §7.5.5, §C.4 (informative); §12.8.1, §12.8.3.2, §12.8.3.3,
§12.8.3.3.1; `CLAUDE.md` principle 3.
Code: `crates/pdf-syntax/src/file.rs` (`FileBytes::from_handle`, `FileBytes::descriptor`),
`crates/pdf-sandbox/src/lockdown_linux.rs` (`PERMITTED_INTERPRETER_EXTRA`, `DESCRIPTOR_LIMIT`),
`crates/viewer-confined/src/protocol.rs` (`open_kind`, `command_descriptor`,
`decode_command_holding`, `ProtocolError::NoDescriptor`), `crates/viewer-confined/src/worker.rs`
(`Source`, `Link`, `read_frame`), `crates/viewer-confined/src/lib.rs` (`ToWorker`, `write_frame`),
`crates/viewer-ui/src/bin/pdf-viewer-confined.rs` (its three opens),
`crates/pdf-model/src/cms.rs` (`Digest::hasher`, `Hasher`), `crates/pdf-model/src/signature.rs`
(`each_signed_window`, `signed_digests`, `RangeProblem`, `Integrity::RangeNotReadable`,
`Authenticity::RangeNotReadable`, `SIGNED_WINDOW`), `crates/viewer-core/src/notes.rs`
(`losses`, `scan_refused`), `crates/viewer-core/src/open.rs` (`scan_refusal_said`).
Tests: `crates/viewer-confined/tests/confined.rs`
(`a_document_open_on_disk_crosses_as_a_descriptor_and_draws_the_same_page`,
`a_confined_interpreter_can_read_a_descriptor_it_holds_where_its_offsets_point`,
`a_confined_interpreter_cannot_stat_a_descriptor_it_holds`),
`crates/viewer-confined/src/protocol.rs::a_document_on_disk_crosses_as_a_descriptor_and_its_length`,
`crates/pdf-syntax/src/file.rs::a_handed_file_is_read_to_the_length_its_opener_stated`,
`crates/pdf-model/src/signature.rs::a_signed_document_answers_the_same_on_disk_as_in_memory`,
`crates/pdf-model/tests/signatures.rs::every_corpus_signature_answers_the_same_on_disk`,
`crates/viewer-core/src/notes.rs::a_scan_the_process_could_not_hold_the_file_for_is_said_with_its_length`.
Measured by: `crates/viewer-confined/examples/confined_peak.rs` (both routes,
`CONFINED_PEAK_ROUTE=whole` for the old one), and
`crates/pdf-model/src/signature.rs::the_window_a_signed_range_is_digested_through_is_priced`
(ignored; a measurement).
Opened by ADR 0809's last consequence and `doc/todo/10`'s three "what is left" items.

## Context

ADR 0809 made `pdf_syntax::FileBytes` a file open on disk, read where the document's offsets
point, so that a 6 GB document opens at the cost of its trailer, its table and page one. It left
three things named rather than done, and this round takes them in the order it named them.

**The confined viewer still read the file whole.** `pdf-viewer-confined`'s worker has no file
system — seccomp-BPF and Landlock, principle 3 — so the document reached it as bytes inside
`Command::Open`, and the host read the whole file to send them. Measured first, with
`examples/confined_peak` opened on the route as 881 left it (`CONFINED_PEAK_ROUTE=whole`), under
`tools/bounded.sh --data 12`:

| document | host `VmHWM` | worker `VmHWM` | outcome |
|---|---|---|---|
| ISO 32000-2, 19.2 MB, 1023 pages | 39.8 MB | 43.7 MB | 1023 pages |
| `poppler-44085-1.xz-0.pdf`, 6.0 GB, 2000 pages | **aborted at 10.44 GiB resident** — `memory allocation of 12003851264 bytes failed` | — | nothing |

The second row is worse than 881's sentence said. The host held the file whole (6 GB) and then
the wire's encoder copied it into a `Vec` whose doubling growth asked for 12 GB, and the process
aborted *in the host*, before the worker had been given the chance to refuse the message by name
(its budget is half its 4 GiB ceiling less a page, ADR 0597). So through the confined viewer the
6 GB document did not open at all, on this machine, under a 12 GiB bound — and on a machine with
the room, the host would have held two copies to send one.

**A signature's digest read the ranges into memory.** `Signature::signed_bytes` took §12.8.1's
`/ByteRange` pairs as slices, which in memory borrowed and on disk read each pair whole — and the
pairs of a signed document are every byte of it but the hole. `cms::Digest::compute` took slices,
so there was no way to feed it a file.

**Nothing consumed `Document::scan_refused`.** 881 built the refusal — a scan the process cannot
hold the file for is recorded once, and an object the scan would have found is `Object::Null` —
and no host said so to a person.

## Decision

### 1. The document crosses as its open file's descriptor, beside `Command::Open`

`Command::Open` still carries a `FileBytes`, and the wire now writes it two ways. Bytes in memory
cross as bytes, whole, as they did: a test's fixture, and §7.11.4's extracted file that a host
hands straight back as a document. **A file on disk crosses as its length and its descriptor**:
the frame carries `open_kind::ON_DISK` and a `u64`, and the host sends the open file's descriptor
beside the frame's nine-byte header with `sendmsg` and `SCM_RIGHTS`. The worker's `recvmsg` of
the header delivers the kernel's duplicate, `decode_command_holding` claims it for the one
command that names it, and `FileBytes::from_handle(File::from(descriptor), length)` is the same
open file at the same length — read where the document's offsets point through the window reader
ADR 0809 built, unchanged.

**Over the transport, not at spawn.** The instruction named both roads. A descriptor inherited at
spawn was refused for three reasons, each sufficient: a worker opens several documents over its
life and `Resuming` reopens one after a worker dies, so one-descriptor-per-spawn is one worker per
document; the standard library marks every descriptor it opens close-on-exec, so inheriting one
means clearing the flag and then `from_raw_fd` on a number passed in `argv` — an `unsafe fn` in a
crate that forbids `unsafe`; and the number would be a claim rather than a handle. Over a socket
the kernel hands the worker an `OwnedFd` through `rustix::net::recvmsg`, which is a safe function,
and the worker never sees a number at all.

**So the worker's standard input is one end of a socket pair rather than a pipe** — `UnixStream::pair`
in the host, the worker's end given as `Stdio::from(OwnedFd)` — because a socket is the only
transport the kernel passes a descriptor over. What the worker writes back is still a pipe; nothing
crosses that way but bytes. The frames are unchanged: the same header, the same payload, written
in the same two calls; a worker run by hand on a pipe gets `ENOTSOCK` from its first `recvmsg` and
reads with `read` from then on, which is an answer and not a kill.

**The length crosses rather than being asked for**, and that is a decision about the filter. The
worker could learn the length with `fstat`, and `File::metadata` does — through `statx`, which
takes a *path* argument: a filter admitting it would let a confined process ask whether
`/etc/passwd` exists. `std::fs::File::read_to_end` asks `statx` and `lseek` both, which is why every
byte the worker reads goes through `pdf_syntax`'s own positional reader (`FileExt::read_at`) and
none through `File::read`. The length is what the host measured when it opened the file, and a
file that has changed since reads as ADR 0809 says one that shrank or grew does.

### 2. The filter admits two more system calls, and each is a fact about a descriptor the worker holds

`Profile::Interpreter` gains `recvmsg` and `pread64`, and nothing else:

- **`recvmsg`** is how the descriptor arrives, on the one socket the worker holds and did not
  create. `socket`, `socketpair`, `connect`, `bind` and `accept` stay off the list, so
  `a_confined_interpreter_cannot_reach_the_network` holds exactly as it did: what `recvmsg` admits
  over `read` is the ancillary data — a descriptor the host chose to send — on standard input.
- **`pread64`** is how the document is read: one positional read at an offset, no cursor moved. On
  the three descriptors the worker inherits — a socket and two pipes — it is `ESPIPE`.

Not `openat`, not `statx`, not `fstat`, not `lseek`. Two probes pin the shape against the kernel
rather than against the list: `a_confined_interpreter_can_read_a_descriptor_it_holds_where_its_offsets_point`
reads five bytes at offset zero of a file opened before the confinement and gets `%PDF-`; and
`a_confined_interpreter_cannot_stat_a_descriptor_it_holds` calls `metadata()` on the same file and
is killed by `SIGSYS`. **The second is the test that fails if somebody admits `statx` "for the
file's length"**, and it is the reason the length crosses on the wire. Run against a filter
without `pread64` (trap 13), the first probe and the on-disk document test both fail; the
`SIGSYS` and `SIGXFSZ` tests in the file are untouched and still pass.

**A descriptor to one file is not a file system.** Landlock restricts what a path can *open*, and
an open file the kernel handed over is usable for what it was opened for: the host opened it
read-only, so `write` on it is `EBADF` and `RLIMIT_FSIZE` of zero is behind that; `mmap` of it,
which the allocator's `mmap` already admits, is another way of reading the same bytes. What the
worker can do with the file it was given is read it, which is what it was given it for.

**`RLIMIT_NOFILE` stays at eight**, and the arithmetic is written on the constant: three inherited
and one per open document, so five documents in one confined viewer at once, and a sixth's
descriptor is dropped by the kernel with `MSG_CTRUNC` — which the worker reads as
`ProtocolError::NoDescriptor`, a refusal by name, and never as a document. A document that is
closed gives its descriptor back. The number is not raised, because no host on this boundary
holds six documents in one worker and a number raised for a shape nobody has is a number
somebody will later have to explain.

### 3. The host's whole-file route is gone

`pdf-viewer-confined` opens the file with `FileBytes::on_disk` at all three of its opens — launch,
the password retry, and `Resuming`'s reopen after a worker dies — and holds no byte of it. `read_file`
stays in `pdf-syntax` for a host that has to hand bytes on (the `open_cost` and `confined_peak`
instruments' old-route columns, the on-disk comparison test), and no window uses it.

### 4. A signature is digested through a window

`cms::Digest` gains `hasher()`, and `Hasher` is the function in progress — one variant per
algorithm, fed a piece at a time, finished once; `Digest::compute` is now written over it.
`Signature::each_signed_window` walks `/ByteRange`'s pairs in order and reads each through
`FileBytes::read` in windows of `SIGNED_WINDOW`, 64 KiB, and `signed_digests(file, algorithms)`
feeds every hasher every window — **one pass however many digests are asked for**, because
§12.8.3.2's signature states none and `Digest::TRIED_WHEN_UNSTATED` is six of them, which was six
reads of the file for one answer. `integrity` and four of `authenticity`'s five arms go through it;
the fifth is Ed25519, whose RFC 8032 verification takes the message itself rather than a digest,
and for a signature over the document's own bytes with no signed attributes that arm still reads
the ranges resident — the one route that does, said on the function.

**What the reader refuses is refused by name.** `RangeProblem::NotInThisFile` is a pair outside
the file, the condition `Coverage::Malformed` reports and `Integrity::RangeNotInThisFile` always
carried. `RangeProblem::NotReadable` is new: the pair was inside the file's stated length and a
window came back short, because the file shrank under the reader or a read failed — and it
reaches a person as `Integrity::RangeNotReadable` and `Authenticity::RangeNotReadable`, with
`FileBytes::read_failure`'s own sentence beside it in the notes. The distinction is load-bearing:
a digest over fewer bytes than the range names differs from the recorded one and would read as a
*modified document*, which is the one thing this must never say by accident.

**The window was measured.** The committed 19.2 MB document's whole extent as a two-pair range,
SHA-256, on disk, warm: 4 KiB windows 10.35 ms, 64 KiB 9.50 ms, 1 MiB 9.39 ms, 16 MiB 9.63 ms,
and the range held whole 11.15 ms. The window is not the cost; 64 KiB is the size `pdf_syntax`
reads a cross-reference section through and it stands for the same reason.

### 5. A scan the process could not hold the file for is said, once

`viewer_core::notes::losses` — the channel §7.5.7's object-stream losses already use, because both
become known at whichever page first asks, never at open — says `Document::scan_refused`'s
refusal once, with the length: *this file's cross-reference table names an object that is not
where it says, and the scan that would find it needs the whole file in memory — N bytes, which
this process cannot hold — so the file was read as far as its table was right and what the table
misplaces draws as nothing (§7.5.4, §C.4)*. It is an `Event::Reported` on the document, in the
pattern the locked and the pageless document use: a fact about the file, worded once, on the
document's report rather than a page's.

Exercised by hand rather than by a unit test, because provoking the refusal needs a file the
process cannot hold: a 4.6 GB sparse file — a valid one-page document whose table places its
content stream in the zeros — opened through `confined_peak`, whose worker runs under the 4 GiB
ceiling, printed the page's own `Unreachable` refusal and then the sentence above. **The run also
found a cost worth writing down**: the misplaced offset lands in 2.3 GB of NUL bytes, which §7.2.3
makes white space, so ADR 0809's window grew — doubling, the parse having examined everything —
to the rest of the file, and the worker's `VmHWM` reached 2.27 GB before the parse failed and the
scan was refused. A window that grows into white space grows to the file, bounded by the process
and read as the end where `try_reserve_exact` refuses; that is graceful and it is not cheap, and
`doc/todo/10` carries it.

## What it cost, and what it bought

### Re-measured

The same instrument, the same documents, both routes in one binary:

| document | route | host `VmHWM` | worker `VmHWM` | outcome |
|---|---|---|---|---|
| ISO 32000-2, 1023 pages | whole | 39.8 MB | 43.7 MB | 1023 pages |
| | **descriptor** | **2.6 MB** (its start-up size) | **25.1 MB** | 1023 pages |
| `poppler-44085-1`, 6.0 GB, 2000 pages | whole | aborted at 10.44 GiB resident | — | nothing |
| | **descriptor** | **2.7 MB** | **22.2–22.5 MB** | **2000 pages, page one drawn, in about a second** |

The worker's `VmPeak` — the counter the ceiling compares against — is 148 MB on every row, which
is its start-up size (`doc/todo/15`), so the document no longer moves it at all.

### What it cost

- The worker's standard input is a socket. Two system calls on the interpreter's allow-list, each
  argued above and each pinned by a probe. A `rustix` dependency on `viewer-confined`, Unix-only,
  with the `net` feature — `rustix` was already the workspace's, and the standard library's
  ancillary-data API is nightly-only.
- One wire tag on `Command::Open`, one `ProtocolError` variant, one `Integrity` variant, one
  `Authenticity` variant, one `RangeProblem` type, one `Hasher` type. The `confined_wire` fuzz
  target's `wire::command` still decodes with nothing beside the bytes and refuses the on-disk
  form by name.
- Off Unix the on-disk form does not cross: `encode_command` refuses it as `Uncarried`, saying
  why, and a Windows host hands bytes. `doc/todo/35` is where that platform's confinement lives.
- `Signature::signed_bytes` is private and used by one arm.

## Consequences

- A document open on disk costs the confined viewer what it costs the other three windows: its
  trailer, its table and page one — in the worker, behind the filter, with the host holding
  nothing. The 6 GB document opens through the confinement.
- The confined worker holds one descriptor per open document and can still name no path; what it
  may do with a descriptor is read it, and the filter says so in two system calls.
- A signature over a file of any size is checked through 64 KiB of it at a time, and a file that
  would not give its bytes is refused as unreadable, never reported as modified.
- A scan the process could not hold the file for reaches a person, once, with the length.
- `doc/todo/34` §5's sentence that `SCM_RIGHTS` "needs `socketpair`, `sendmsg` and `recvmsg` on
  the interpreter's allow-list, and `a_confined_interpreter_cannot_reach_the_network` is the test
  that would have to be weakened" was wrong on both counts and is corrected: the host makes the
  pair and sends, the worker only receives, and the network test is untouched.
- Left, and written in `doc/todo/10`: the whitespace-window growth above; the Ed25519 arm's
  resident read; a Windows confinement, which `doc/todo/35` owns.
