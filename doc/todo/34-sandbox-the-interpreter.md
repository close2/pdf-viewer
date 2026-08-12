# Confine the interpreter and the rasteriser

Status: **built, answers every question, stoppable, and not yet where a person would meet it.** The
confined process exists, draws real pages (ADR 0218) and carries all **twenty-nine** questions —
twenty-five since the three-hundred-and-eighty-sixth (ADR 0223), the caret's inverse and a field's
selected range since the three-hundred-and-eighty-eighth (ADR 0225), §12.7's form since the
three-hundred-and-ninety-eighth (ADR 0235) and §12.5.6.6's free text at a point since the
four-hundred-and-first (ADR 0238); a hostile document has a cancel since the
four-hundred-and-fourth (ADR 0241). **This line said twenty-eight until the
four-hundred-and-forty-fifth counted them.** What is left is that the window does not use it.
Priority: 34
Clauses: —, this is `CLAUDE.md` principle 3
Code: `crates/viewer-confined`, `crates/pdf-sandbox/src/lockdown.rs`

## What exists since the three-hundred-and-eighty-first session

`pdf-view-worker` holds a `viewer_core::Viewer` and a `render_cpu::CpuRasterizer` behind
seccomp-BPF, Landlock and a 4 GiB address-space ceiling, with no filesystem and no network.
`viewer_confined::Confined` is the host side: `Command` in, `Event` out, `Query` → `Reply`, and the
rasters cross as pixels because the confined process owns the rasteriser. `pdf_sandbox::lockdown`
grew a `Profile`, so the decoder's allow-list is untouched and the interpreter's is a second one,
four system calls longer.

What the tests establish, on this kernel: a page byte-identical to the one drawn in process; a page
turn and a magnification; a JBIG2 document decoded *inside* the confinement; a confined process that
cannot open a file, cannot open a socket and cannot start a program; and `Confinement::shortfall`
answering `None` because everything was enforced. **Since the four-hundred-and-fourth**: a document
that will not finish, cancelled from another thread, and a warmed allocator that never asks the
kernel again. `examples/confined_page` and `examples/confined_cancel` are what a person runs.

## What the three-hundred-and-eighty-sixth session added

`protocol/panels.rs`: the eleven answers a panel is made of, encoded field for field, with a round
trip apiece and a comparison against the same answer read in this process on three real documents.
**Twelve since the three-hundred-and-ninety-eighth**, the twelfth being §12.7's form — compared the
same way, on `issue17492.pdf`, and with an edit built out of what crossed sent back through the same
pipe (ADR 0235).
`examples/confined_panels` prints a sidebar's worth of a document out of the confinement. The
transport's stand-in test became `fuzz/fuzz_targets/confined_wire.rs`, clean at 44 723 045 runs,
and clean again at **13 175 908 runs in 241 s** in the four-hundred-and-fourth, which changed how
a frame header is written and therefore owed it.
ADR 0223 has the argument, the measurements and what it refuses.

## What is left, in the order it matters

### 1. Two things the panel answers left behind, and neither is a hole in the transport

**~~§12.3.5.1's `/D` is decided by nobody.~~ Closed in the three-hundred-and-ninety-fourth session**
(ADR 0231). `Answer::Collection` carries the resolved `Initial` beside Table 153, the protocol
encodes its four cases, and `viewer_ui::chrome` sets the initial document's row in bold while an
empty tree says so instead of drawing nothing. It took exactly what this entry predicted — a field
on the answer and a consumer for it — and it was never a hole in the transport: `viewer_ui::chrome`
was in the same position in the same process. Nothing is `#[non_exhaustive]`, so the change broke
every consumer's build, which is what that is for.

**An outline row cannot say which page it goes to.** `Item::destination` crosses, but turning a
`Destination` into a page index is `page_index_with(document, pages, …)` and a host has neither.
This is *not* a gap the transport made: `viewer-ui` is in the same position and the answer for both
is `Command::Activate(item.id)`, which follows §12.6's action machinery inside the core and crosses.
Worth knowing rather than fixing; it becomes a question the day a panel wants to print a number
beside a row.

### 2. The window is a tier-2 host and this boundary is tier 1

`viewer-ui` hands back `Rendered::Presented` and draws on the graphics device; a confined process
draws on the processor and hands over a raster. Putting the window on this boundary is therefore a
change of *tier*, not a change of transport — and `CLAUDE.md` says page one goes to the graphics
device, by the owner's decision. Two ways out, and neither has been argued:

- the host keeps the device and the confined process ships **display lists** rather than pixels,
  which is the two-protocol design section 0 was glad to be rid of;
- or the confined process is given a window handle and drives the device itself, which needs
  `wgpu` inside the confinement — a large surface, and drivers open files.

Until one of them is settled, the confined path is for hosts that want pixels.

### 3. ~~A hostile document has no deadline~~ — closed in the four-hundred-and-fourth session

`Canceller` is the answer and **a cancel is a kill** (ADR 0241): the confined process is
interpreting a hostile document, so a cancel it has to *agree* to is a cancel the document can
decline, and the only one worth the name is the one the kernel enforces. `Canceller::cancel` ends
the worker from any thread, whichever call the host was blocked in returns
`ConfinedError::Cancelled`, and `Confined::start_with` takes a canceller made before there is a
worker — because `start` blocks on the greeting too, which is what `doc/todo/01`'s fifth sweep found
when it was run over this crate's own surface.

Demonstrated on a **1567-byte** document that draws for **44.2 s**: the host's thread comes back in
**0.83–1.97 ms** over six runs. `tests/confined.rs` and `examples/confined_cancel`.

**What is still not here is a deadline, and deliberately.** A page's cost is bounded by the
document and the magnification together, so a fixed number refuses work a viewer permits. What a
host has is the ability to decide, on its own grounds.

### 4. One rasterising thread — repriced, and the `glibc` claim is now measured

ADR 0218 section 2: `glibc`'s allocator sizes its arena count from `__get_nprocs()`, which reads
`/sys/devices/system/cpu/online`, so a thread's first allocation in a many-threaded confined
process is an `openat` the filter kills for.

**What it is worth was wrong by an order of magnitude, and the reason is which page it was measured
on.** This entry said "about 1 ms of the 7 ms this page takes". `pdf-model`'s `strip_spans`, run in
the four-hundred-and-fourth session (ADR 0241 section 6):

| page | 1 strip | best | strips the geometry grants |
|---|---|---|---|
| `PDF20_AN001-BPC.pdf` p1 at 1× | 2.2 ms | 1.3 ms | **2**, whatever is asked |
| the same at 2× | 8.2 ms | 5.8 ms | **2** |
| ISO 32000-2 p101 at 1× (3007 commands) | **19.9 ms** | **7.2 ms** | 8, and 11 at 16 asked |
| the same at 2× | **31.0 ms** | **12.7 ms** | 15 at 16 asked |

So it is a millisecond on a sparse page — where ADR 0139's constrained split grants two strips and
no thread count can beat that — and **twelve of twenty milliseconds** on a dense one. A page turn
pays this every time, which puts it above item 5 for interactivity.

The two ways to get the cores back:

- **Per-thread Landlock plus an allocator warm-up.** Build the pool *before* seccomp with a
  `start_handler` that puts each worker in its own Landlock domain and allocates once.
  **The allocator half is measured and it holds**: `tests/confined.rs`'s
  `an_allocator_warmed_before_the_filter_does_not_ask_the_kernel_again` warms 24 threads, confines,
  draws a page on 24 strips and then broadcasts twenty rounds of 4 MiB allocations to every thread;
  `strace` counts 25 `clone3` before the filter, none after, and **no `openat` after it at all**.
  What is left is the Landlock half, and it is concrete: `pdf-sandbox` has no entry point that
  applies Landlock alone to the calling thread, so a `start_handler` cannot put its worker in the
  domain. Until it has one, a warmed pool has the seccomp filter (installed with `TSYNC`) and not
  the depth layer, which is what ADR 0218 rejected and still rejects.
- **An allocator that does not consult the filesystem.** Every candidate this project has looked
  at contains `unsafe`, which is a separate decision.

### 5. The document crosses as bytes — and the pipe is a tenth of what it was blamed for

This entry said 19.2 MB of ISO 32000-2 down a pipe "is most of the 67 ms that document takes to
reach its first page", and proposed a `memfd` or an `SCM_RIGHTS` descriptor. The first half is
right and the reason was wrong (ADR 0241 section 5). Measured with `examples/confined_page`'s new
**ballast** line — a valid one-page document padded to exactly the real one's length with a stream
nothing refers to, so that what is timed is the transport and nothing else:

| | |
|---|---|
| ISO 32000-2 opened, interpreted and drawn, confined | 65–108 ms |
| **19.2 MB of ballast, crossed and drawn blank** | **41–66 ms** |
| 0 bytes of ballast | 1.2–2.3 ms |
| the same 19.2 MB through a bare pipe (`dd \| dd`) | **3.7, 4.9, 5.5 ms** |

**The kernel's pipe is four milliseconds of it.** The rest is five passes over the document on our
side: the encoder's buffer, a header put in front of it by building a third buffer, the two the
pipe makes, the worker's frame allocation and `decode_command`'s copy into `Command::Open`.

**One of those was free to remove and is gone**: both ends write the nine-byte header and the
payload in two calls instead of concatenating. Nine runs each way — the 4.1 MB raster falls from
4.32/5.64 ms (min/median) to **3.23/3.74**, seven of nine "after" samples below the "before"
minimum; the 19.2 MB open moved by less than its spread and is not claimed.

**The mapping is still a decision and now a sharper one.** `memmap2::Mmap::map`,
`memmap2::MmapOptions::map_copy_read_only` and `rustix::mm::mmap` are all `pub unsafe fn` —
soundly, since a mapping's bytes can change under the reader — while `rustix::fs::memfd_create` is
safe. So *making and writing* a `memfd` needs no `unsafe` and *mapping* one does, and
`viewer-confined` is `#![forbid(unsafe_code)]` and holds a whole document. No dependency hides it
behind a safe signature; the construction that would justify one is a **sealed** `memfd`
(`F_SEAL_WRITE | F_SEAL_SHRINK | F_SEAL_GROW`), which would be a new crate in this workspace that
seals, passes, maps and hands out `&[u8]` and parses nothing — and whether such a crate falls under
principle 3's rule is the question to answer out loud rather than assume.

And getting the descriptor across has its own cost. The document arrives *after* the spawn, so an
inherited descriptor means one worker per document; the runtime alternative, `SCM_RIGHTS`, needs
`socketpair`, `sendmsg` and `recvmsg` on the interpreter's allow-list, and
`a_confined_interpreter_cannot_reach_the_network` is the test that would have to be weakened to get
them. Three system calls for latency is the wrong direction.

## Two things that stayed true and are worth keeping

- **`viewer-core` needed no change at all.** Rules 2, 3 and 4 — no filesystem, no clock, no threads
  it was not handed — describe a confined process exactly, which is why this cost a transport
  rather than a redesign.
- **The sandbox is a flag and the default is the safe one.** There is no path in `viewer-confined`
  that interprets a document in the calling process when the worker will not start.
