# Confine the interpreter and the rasteriser

Status: **built, answers every question, stoppable, its tier question settled, and not yet where a
person would meet it.** The
confined process exists, draws real pages (ADR 0218) and carries all **twenty-nine** questions —
twenty-five since the three-hundred-and-eighty-sixth (ADR 0223), the caret's inverse and a field's
selected range since the three-hundred-and-eighty-eighth (ADR 0225), §12.7's form since the
three-hundred-and-ninety-eighth (ADR 0235) and §12.5.6.6's free text at a point since the
four-hundred-and-first (ADR 0238); a hostile document has a cancel since the
four-hundred-and-fourth (ADR 0241). **This line said twenty-eight until the
four-hundred-and-forty-fifth counted them.** **A window uses it since the
seven-hundred-and-seventy-fifth** — `pdf-viewer-confined`, ADR 0713, deliberately the smallest
complete host — and what is left is that the three *established* windows do not.
Priority: 34
Clauses: —, this is `CLAUDE.md` principle 3
Code: `crates/viewer-confined`, `crates/pdf-sandbox/src/lockdown.rs`

## What exists since the three-hundred-and-eighty-first session

`pdf-view-worker` holds a `viewer_core::Viewer` and a `render_cpu::CpuRasterizer` behind
seccomp-BPF, Landlock and a 4 GiB address-space ceiling, with no filesystem and no network.
`viewer_confined::Confined` is the host side: `Command` in, `Event` out, `Query` → `Reply`, and the
rasters cross as pixels because the confined process owns the rasteriser. `pdf_sandbox::lockdown`
grew a `Profile`, so the decoder's allow-list is untouched and the interpreter's is a second one —
four system calls longer when it was written, seven since ADR 0812's descriptor, and since ADR 0888
one more that is **narrower than a system call number**: `fcntl` for the single command
`F_GETFD`, and a kill for every other. Its own constant says why, and
`crates/pdf-sandbox/src/lockdown_linux.rs` is the list rather than this sentence.

What the tests establish, on this kernel: a page byte-identical to the one drawn in process; a page
turn and a magnification; a JBIG2 document decoded *inside* the confinement; a confined process that
cannot open a file, cannot open a socket and cannot start a program; and `Confinement::shortfall`
answering `None` because everything was enforced. **Since the four-hundred-and-fourth**: a document
that will not finish, cancelled from another thread, and a warmed allocator that never asks the
kernel again. `examples/confined_page`, `examples/confined_cancel` and
`examples/confined_peak` are what a person runs.

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

### 2. ~~The window is a tier-2 host and this boundary is tier 1~~ — settled in the seven-hundred-and-twenty-fourth session, the codec built in the seven-hundred-and-thirty-second and wired in in the seven-hundred-and-thirty-sixth

`viewer-ui` hands back `Rendered::Presented` and draws on the graphics device; a confined process
draws on the processor and hands over a raster, so putting the window on this boundary is a change
of *tier* rather than of transport. This entry named two ways out and argued neither. **ADR 0607
argues both and the answer is display lists**, with the raster payload kept and chosen per page by
size. What is owed now is the codec, not the decision.

**The second way out — a window handle and `wgpu` inside the confinement — does not exist**, and
that is measured rather than reasoned. `render-quorra`'s `examples/device_under_confinement`
brings a real device up and confines the process holding it: a device confined *before* drawing
dies on its first frame, and a device confined after a frame has already been drawn dies on the
*same* frame drawn again, both with **SIGSYS**, both on `ioctl(DRM_IOCTL_AMDGPU_GEM_CREATE)` — the
first system call after the filter goes on. A graphics device is a conversation with a kernel
driver, and no ordering makes it stop needing `ioctl`. The process also holds **9 descriptors
against the confinement's ceiling of 8**, so `landlock_create_ruleset` fails `EMFILE` and the depth
layer is gone before the filter is reached. This entry's "a large surface, and drivers open files"
now has counts: **55 distinct system calls in a bring-up, 35 of them off the interpreter's 28-call
allow-list**, `/dev/dri/renderD128` with 25 distinct DRM request numbers and about 190 ioctls a
frame, the shader cache read *and written*, 56 driver manifests parsed, and a socket connected to
`/tmp/.X11-unix/X0` on a headless run.

**The codec is built and the number is now a measurement** (ADR 0626).
`viewer_confined::wire::{encode_display_list, display_list, crossing}` is both sides, with four
tables interned by `Arc` identity — paths, image samples, shading *kinds* and shadings, the third
of which ADR 0607's accounting did not separate and which is what turns
`bug1721218_reduced.pdf`'s 3576 mesh paints back into three geometries. The tables precede the
body, so every identifier is bounds-checked against a length the decoder read rather than one the
message asserts; a clip table that cannot be rebuilt as the message numbers it is refused, because
`add_clip` deduplicates and a region stated twice would renumber every identifier after it.
`fuzz/fuzz_targets/display_list.rs` is the target, and `examples/list_over_the_wire` is the
instrument that replaces the prediction below with what the encoder writes.

**And the wiring is done** (ADR 0633). `viewer_confined::Framed::payload` is the marks or the
pixels; `viewer-confined` takes no rasteriser, because the device is the host's by necessity and
ADR 0607's own sentence points at `viewer-ui`; the target crosses beside the marks so that no host
rebuilds one out of a page size and a scale; and `MAGIC` moved once, `PDFVCF03` → `PDFVCF04`. The
decoder refuses a target past `viewer_core::MAX_PIXELS`, which is the one length on this boundary
with no bytes behind it — eight bytes that become the host's own allocation, which is the
seven-hundred-and-nineteenth session's finding arriving somewhere new.

**And the render it was wasting is gone since the seven-hundred-and-fortieth** (ADR 0640). The one
`viewer-core` change this whole item needed is `Rendered::Listed` — *the host took this request's
own list*, said about a page rather than about the viewer, so `MAX_PIXELS` goes on bounding a
confined process's raster and `Query::Frame` goes on answering for the pages that must still cross
as pixels. `viewer_confined::protocol::Marks` holds each page's origin now, from
`Query::PageGeometry`, because the viewer holds nothing at all for a page it was told the host
took; `encode_answer` merges the two halves and the wire format did not move.

**What that changes about item 3's cancel is a sentence, not a hole**: a cancel stops the work
*this process* does, so on the marks arm it covers the interpretation and there is no rasterisation
of ours to stop. Drawing the marks was always the host's — the worker had merely been doing it
twice and discarding one.

**What decided the first way out is one number this entry never had**: how big a display list is
beside the pixels it produces. `viewer-confined`'s `examples/list_against_raster` is that
instrument — byte counts, so it is load-immune — and over `doc/pdf.js`'s first pages a list is
**about 2% of its raster at the median** at a window's scale, exceeding it on **4%** of pages,
which are the scanned ones: a scan's decoded samples *are* its display list. Hence the per-page
choice. Two constraints came with it and are the codec's, both with numbers in ADR 0607: the
encoder **must** preserve `Arc` identity or it buys nothing, and `ImageSource::AtDeviceScale` and
`ShadingKind::Sampled` carry trait objects that cannot cross as they stand — 4 of 958 first pages,
covered by the raster arm.

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

**And since ADR 0640 the cancel's scope is worth stating exactly**, because it is easy to read as
larger than it is: it ends the worker, so what it stops is what the worker is doing. On the pixel
arm that is the interpretation *and* the rasterisation; on the marks arm the worker does not
rasterise, so it is the interpretation alone and the drawing is the host's.

**The other half of that sentence has an answer since the seven-hundred-and-forty-fifth session,
and it is a different object with a different name** (ADR 0650). `pdf_render::Interrupt` is
*raised* and *honoured* where a `Canceller` *ends a process*, and the reason a cooperative flag is
enough there and not here is one line: on the host's side **the loop is ours**. A hostile document
arrives in `render-cpu` as a `DisplayList`, which is data — it can make the command loop long, and
it cannot make an iteration of it decline to check. Inside the confinement there is no such loop
to appeal to, because what is running is the document's own content stream.
`doc/todo/15` carries what the host still owes, which is the policy rather than the mechanism.

`tests/support/amplification.rs` is five levels deep rather than four for
this reason and says so: four levels' marks are smaller than a window's pixels, so that document
crossed as marks and there was nothing left for the cancel test to cancel.

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

**The last two of those are still both paid, and what changed in the six-hundred-and-nineteenth is
how long they overlap** (ADR 0597). The worker's frame buffer used to be held across the whole of
the work, so it was alive beside `Command::Open`'s copy *and* beside the `Arc<[u8]>` `pdf_syntax`
makes — three copies of the document at the peak, measured as start-up plus exactly 3× its length.
It is dropped the moment the message is decoded now, which is two. That is address space rather
than time: the passes are unchanged and so are the figures above.

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

~~And getting the descriptor across has its own cost. The document arrives *after* the spawn, so an
inherited descriptor means one worker per document; the runtime alternative, `SCM_RIGHTS`, needs
`socketpair`, `sendmsg` and `recvmsg` on the interpreter's allow-list, and
`a_confined_interpreter_cannot_reach_the_network` is the test that would have to be weakened to get
them. Three system calls for latency is the wrong direction.~~ **The first half of that was right
and the second was wrong twice, and the descriptor crosses since the eight-hundred-and-eighty-third
session** (ADR 0812) — for memory rather than latency: a 6 GB document could not cross as bytes
at all. The *host* makes the socket pair and sends; the worker only receives, so what its
allow-list gains is `recvmsg` on a socket it did not create and cannot create, plus `pread64` on
the file it was handed — and `a_confined_interpreter_cannot_reach_the_network` is untouched, with
two probes beside it pinning that the worker can read the descriptor and cannot `stat` it. The
`memfd` question above is unchanged: that was about *mapping*, and nothing here maps.

**A descriptor a worker is handed is also a descriptor it has to give back, and that cost was found
forty-one sessions later** (session 924, ADR 0888). `OwnedFd::drop` asks `fcntl(fd, F_GETFD)` before
`close` under `core::ub_checks::check_library_ub()`, so closing a document killed every build with
library-UB checks compiled in while the release build passed — the shape trap 32 is about. The
allow-list gains one *command* for it, narrowed by argument, on the interpreter profile alone; the
alternatives were both leaks and `DESCRIPTOR_LIMIT` is the arithmetic that refuses them. Five probes
now stand where two did: the worker can read the descriptor, cannot `stat` it, can ask `F_GETFD`
about it, cannot `F_SETFD` it, and cannot duplicate it — and a decoder cannot ask any of them.

## Two things that stayed true and are worth keeping

- **`viewer-core` needed no change at all.** Rules 2, 3 and 4 — no filesystem, no clock, no threads
  it was not handed — describe a confined process exactly, which is why this cost a transport
  rather than a redesign.
- **The sandbox is a flag and the default is the safe one.** There is no path in `viewer-confined`
  that interprets a document in the calling process when the worker will not start.
