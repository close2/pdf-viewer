# 902 — Two workers, one wire, and a generator that is killed for asking

2026-09-03. Argued in [ADR 0846](../adr/0846-two-workers-one-wire-and-the-line-between-a-transport-and-a-vocabulary.md)
and [ADR 0847](../adr/0847-the-vfs-worker-needs-no-system-call-the-viewer-does-not-and-one-it-was-about-to.md).
The **second** implementation round of [RFC 0003](../rfc/0003-file-system-faces.md), on round 899's
branch because it continues that landing; `main` had moved under round 901 and the merge of
round-899 and was merged in before the gates ran.

`doc/todo/58` §4 was the whole scope, in its own words: "**No face ships before it exists**: a
mount is entered by anything that touches a folder, and a file manager will open a document nobody
chose to open." So this round built RFC §6's confined worker, and the thing it had to decide first
was what the *second* confined worker in this tree is allowed to copy from the first.

Touched: **`crates/confined-transport/`** (new — `frame.rs`, `greeting.rs`, `channel_unix.rs`,
`channel_pipe.rs`, `link.rs`, `supervision.rs`, `host.rs`, `ceiling.rs`);
**`crates/viewer-confined/`** (moved onto it: `lib.rs` loses 600 lines, `protocol.rs` and
`worker.rs` delegate, `rustix` leaves the manifest); **`crates/pdf-vfs/`** (`wire.rs`, `serve.rs`,
`confined.rs`, `src/bin/pdf-vfs-worker.rs`, `tests/confined.rs`, `examples/vfs_cost.rs` all new;
`worker.rs` and `lib.rs` changed); `crates/pdf-transform/src/render.rs` (`RenderPlan::strips`) and
the nine places that construct one; `Cargo.toml` and `Cargo.lock`; `doc/rfc/0003-…`,
`doc/todo/58-…`, `doc/todo/README.md`, `doc/todo/02-every-round.md`, `doc/state-of-play.md`,
`doc/crate-map.md`; two ADRs, this file.

## 1. One wire, and where the line between two protocols runs

`viewer-confined` is 7 500 lines and about 600 of them are wire: a frame header, a greeting, a
socket that passes a descriptor, a child that can be killed at any moment, and the arithmetic from
an address-space ceiling to a message budget. None of that is about a page. So it is
`crates/confined-transport` now, and **`viewer-confined` was moved onto it in the same round** —
a shared thing nobody has moved onto is a third copy. Its whole suite, thirty-three tests including
every confinement probe, passes unchanged.

The line is one function. **`frame::parse_header` answers a length and hands the kind back
untouched**; each protocol matches the byte against its own set. A transport that validated the
kind would have to hold both protocols' discriminants — and the version this was extracted from
*did* validate it, which is why the line is worth naming. The magic is the caller's for the same
reason: `PDFVCF05` and `PDFVFS01` are two protocols on one wire, and a host that got the wrong
worker back refuses at nine bytes rather than at the first answer it misreads.

## 2. The allow-list, and the defect it found

**`Profile::Interpreter`, unchanged. Not a third profile.** The vfs worker parses a document handed
to it as a descriptor, draws pages on one rayon thread, decodes its own codecs in-process and
writes into `MemorySinks` — which is what `pdf-view-worker` already does, so it issues nothing that
list does not already carry. Measured with `strace -ff` rather than reasoned: twenty-three distinct
calls after the filter, every one of them on `PERMITTED` or `PERMITTED_INTERPRETER_EXTRA`.

And the filter found something on the way, which is the round's one real finding.
**`pdf_transform::render` drew with `CpuRasterizer::new()`, which asks the machine how many cores
it has** — an `openat` of `/proc/self/cgroup` that a confined process is killed for. `render-cpu`'s
own comment says a confined caller states the number with `with_strips`; nothing in the transform
suite could state it. It is not a latent risk but a kill, reproduced: with the field left `None`
the round's own comparison test fails with *"the confined worker stopped without answering (killed
by signal 31 (SIGSYS: a system call the confinement forbids))"* on the 150 dpi render and on
nothing before it. `RenderPlan` gained `strips`, `None` everywhere the suite already was.

## 3. What was looked at

ADR 0812 for the `SCM_RIGHTS` route and why `statx` stays off the list even for the file's own
length; ADR 0218 for why a confined process decodes its own JBIG2 and why it rasterises on one
thread; ADR 0597 for the two numbers under the message budget and for the worker's standard error
being a pipe; `pdf_sandbox::lockdown`'s own rule that a third profile means measuring a third;
`doc/PDF20_AN001-BPC.pdf` and `doc/Tagged-PDF-Best-Practice-Guide.pdf` for the comparison and the
measurements; and `doc/PDF-Declarations.pdf`, whose two §7.11.4 embedded files are what says an
inventory and an extraction cross the boundary unchanged under the document's own names.

## 4. Numbers

`crates/pdf-vfs/examples/vfs_cost.rs` is the answer to `doc/todo/58` §5's "[n]othing is measured",
and it is the first number this crate has had. This machine, `--profile gates`, load under 4, best
of five:

| | `PDF20_AN001-BPC.pdf` (5 pages) | `Tagged-PDF-Best-Practice-Guide.pdf` (72) |
|---|---|---|
| a worker per generation, in process | 0 µs | 0 µs |
| a worker per generation, confined | **1.23 ms** | **3.91 ms** |
| page count, in process → confined | 70 µs → 84 µs | 204 µs → 238 µs |
| a page's text | 804 µs → 821 µs | 3.10 ms → 3.30 ms |
| §14.3.3's information | 22 µs → 36 µs | 184 µs → 208 µs |
| a page out (`pages/0001.pdf`) | 681 µs → 723 µs | 10.14 ms → 10.23 ms |
| a page's images | 124 µs → 129 µs | 522 µs → 564 µs |
| 150 dpi render | 9.06 ms → 10.63 ms | 31.67 ms → **23.59 ms** |
| 300 dpi render | 25.77 ms → 31.73 ms | 67.34 ms → **64.53 ms** |
| the largest answer | 112.7 KiB | 126.1 KiB |
| a `stat` that generates, cold → cached | 61.68 ms → 6 µs | 96.26 ms → 8 µs |

The in-process column for a worker is *nothing*, honestly: `InProcessWorkers::spawn` opens no file
and reads no byte, so the whole confined column is what the confinement costs. Per question it is
tens of microseconds — and on the two renders of the longer document the confined side came out
*ahead*, which is what a difference inside the run-to-run spread looks like rather than a finding.
The `stat` figures are RFC 0003 §5.5's rule priced: the first one starts a worker and draws the
page, and the cache answers the next in single-digit microseconds.

The bound past which an answer is refused is derived rather than chosen — a 4 GiB ceiling less what
the process already occupied, less ADR 0597's settling allowance, less a page's pixels, over two
copies — and comes to **1 464 MiB** with the default budget's 2²⁸ pixels.

## 5. Gates

The change→gate map's core, plus everything `pdf-transform` is under, run in this worktree after
`main` was merged in: the transform gate and its five corpus walks, one walk at a time on the
machine, waiting for a neighbouring round's `pages_corpus` to finish first. **The poller matched
the gate *binaries* under a build directory** (`cargo-target/*/gates/deps/<name>-`) rather than the
test names, which is round 899's lesson and the reason two rounds' wait loops do not deadlock
against each other. The results are in the round's report and not here.

## 6. What the next round of this stream does first

`doc/todo/58`'s order, minus the item this round took: **the write side**, whose five meanings the
layout table already states, and then **the FUSE face**, which is the pure-Rust one. What §4 still
owes is three things a face's requirements should shape rather than this stream: whether workers are
pooled, whether the operation that finds a death retries it, and how a `Canceller` reaches through
`Vfs`. And the owner has not been asked RFC §9's seven open questions since approving the document.
