# 883 — The document crosses the confinement as a descriptor, and a signature is digested a window at a time: the confined viewer holds no byte of a 6 GB file, the worker reads it through two system calls and can still name no path, and a scan the process cannot hold is said out loud

Date: 2026-09-03.
ADR: [0812](../adr/0812-the-document-crosses-the-confinement-as-a-descriptor-and-a-signature-is-digested-a-window-at-a-time.md).
Touched: `crates/pdf-syntax/src/file.rs`; `crates/pdf-sandbox/src/lockdown_linux.rs`;
`crates/viewer-confined/Cargo.toml`, `src/lib.rs`, `src/protocol.rs`, `src/worker.rs`,
`tests/confined.rs`, `examples/confined_peak.rs`; `crates/viewer-ui/src/bin/pdf-viewer-confined.rs`;
`crates/pdf-model/src/cms.rs`, `src/signature.rs`, `tests/signatures.rs`,
`examples/signature_algorithm_census.rs`; `crates/viewer-core/src/notes.rs`, `src/open.rs`;
`doc/conformance/ledger.toml` (§7.5.1, §12.8.1, §12.8.3.2, §12.8.3.3, §12.8.3.3.1);
`doc/todo/10-bounds-that-cap-size.md`, `doc/todo/34-sandbox-the-interpreter.md`,
`doc/ui-boundary.md`, `doc/state-of-play.md`; `Cargo.lock`.

## Measured first

`examples/confined_peak` gained the on-disk route and a column for the host's own resident peak,
and was run on the route as 881 left it (`CONFINED_PEAK_ROUTE=whole`) under
`tools/bounded.sh --data 12`. ISO 32000-2 (19.2 MB, 1023 pages): host `VmHWM` 39.8 MB, worker
43.7 MB. `batch5/poppler`'s 6 001 925 614-byte document: **the host aborted at 10.44 GiB
resident** — `memory allocation of 12003851264 bytes failed` — before the worker could refuse the
message by name, because the wire's encoder copied the whole file into a buffer whose doubling
growth asked for twice it. Through the confined viewer that document did not open at all.

## The design

ADR 0812. The document crosses to the worker as its open file's descriptor, sent with `SCM_RIGHTS`
beside `Command::Open`'s nine-byte header over a socket pair the host makes and gives the worker as
its standard input; the frame carries the file's length, and the worker holds the same open file
through `FileBytes::from_handle` and reads it where the offsets point. Over the transport rather
than at spawn: a worker opens several documents and `Resuming` reopens one, and an inherited
descriptor would need `from_raw_fd`, an `unsafe fn`, in a crate that forbids it. The interpreter's
allow-list gains `recvmsg` — on the one socket the worker holds and cannot create, so the network
probe is untouched — and `pread64`, and nothing else: the length crosses on the wire because
`statx` takes a path, and a probe pins that the worker can read the descriptor and is killed for
`stat`-ing it. `RLIMIT_NOFILE` stays at eight, five documents at once, a sixth's descriptor
dropped by the kernel and refused by name. `pdf-viewer-confined` opens on disk at all three of its
opens and holds no byte of the file.

`cms::Digest::hasher` is the function in progress; `Signature::signed_digests` walks `/ByteRange`
through 64 KiB windows and feeds every algorithm asked for in one pass, so §12.8.3.2's six
attempts read the file once. A window the disk would not give is `Integrity::RangeNotReadable`,
kept apart from `Changed`. The window was priced on the 19.2 MB document: 4 KiB 10.35 ms,
64 KiB 9.50, 1 MiB 9.39, 16 MiB 9.63, the range held whole 11.15. Ed25519 over the document's own
bytes still reads the ranges resident, said on the function.

`viewer_core::notes::losses` says `Document::scan_refused` once, with the length, on the
document's report — the channel §7.5.7's losses use, for the same reason. Exercised by hand on a
4.6 GB sparse file through the confined viewer under its 4 GiB ceiling, which printed the
sentence and found that a misplaced offset landing in white space grows ADR 0809's window to the
rest of the file (2.27 GB resident) before the scan is refused; `doc/todo/10` carries it.

## Re-measured

ISO 32000-2 through the descriptor: host `VmHWM` 2.6 MB (its start-up size), worker 25.1 MB. The
six-gigabyte document: host 2.7 MB, worker 22.2–22.5 MB, **2000 pages and page one drawn in about
a second**. The worker's `VmPeak` is 148 MB on every row, its start-up size.

## Gates

The whole `doc/todo/02` §2 sequence under `tools/bounded.sh` (`--tree 8` for a build, `12` for a
walk, one walk at a time beside round 882's work on `main`), every line exit 0 with one exception
on the first pass: formatting and `clippy` under `-D warnings` silent for the workspace and for
`fuzz/`; `nextest` failed one test — `pdf-viewer-confined`'s
`a_file_gone_before_the_retry_is_said_not_fatal` asserted the old wording "cannot re-read" and the
host now says "cannot reopen", the test was brought to the sentence, and the workspace re-run is
**3036 tests passed, 22 skipped**; doctests green; the corpus gate at **974 documents, 64
incomplete**; the oracle at **1945 pages, 1841 complete, 104 incomplete**; the three text gates
green (99.67% of matched words in bounds); the two censuses green (the drag at 98.91% over 453
documents; 877 of 877 untagged pages answering the empty tree); dates 1514 of 1545; XMP and JPEG
2000 green; quorra at **958 pages, 929 agree, 22 differ, 7 refused, 16 not comparable** — three
more refused than round 881 recorded, so the same gate was run on the tree with this round's
patch reversed and its own sandbox worker rebuilt, and printed the identical line: the figure is
the tree's and not this change's; fixed documents **59 of 59**; the transform gate at 194.4
pages/s over a floor of 40; the writer over 974 documents, 941 attached and read back, nothing
unexplained; conformance green, and green again after this file was written. Not a fifth round;
the measurement was taken from this worktree's own release build of `confined_peak` and
`pdf-view-worker`, which is what it measures, and §5's install was not run.

Trap 13 was run on the two new confinement probes: with `pread64` taken off the interpreter's
allow-list, `a_confined_interpreter_can_read_a_descriptor_it_holds_where_its_offsets_point` and
`a_document_open_on_disk_crosses_as_a_descriptor_and_draws_the_same_page` both fail and the rest
of the file passes. The `SIGSYS` and `SIGXFSZ` tests are untouched.
