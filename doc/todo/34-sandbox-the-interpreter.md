# Confine the interpreter and the rasteriser

Status: **built, and not yet where a person would meet it.** The confined process exists and draws
real pages (ADR 0218); the window does not use it, eleven of the twenty-five questions do not cross,
and a hostile document has no cancel.
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
answering `None` because everything was enforced. `examples/confined_page` is what a person runs.

## What is left, in the order it matters

### 1. Eleven questions do not cross, and four sidebar tabs are among them

`Query::Outline`, `Layers`, `Attachments`, `Collection`, `Articles`, `Thumbnail`, `Properties`,
`Opening`, `Preferences`, `Popups` and `AccessibilityTree` answer with `pdf-model` types —
an outline's tree, Table 147 whole, a decoded thumbnail, §14.7's structure. Each is refused **by
name** rather than answered with nothing, so nothing is silently wrong; but a host on this boundary
has no panels. This is the largest single piece and it is ordinary work: one encoding per type,
beside the twenty-eight that are already there, with a round trip apiece.

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

### 3. A hostile document has no deadline, and the reason a decode's would be wrong

`Confined::read_exactly` blocks. A decode has a 30-second budget because one image's cost is
bounded by its own dimensions; a page's is bounded by the document *and* the magnification, so any
fixed number refuses work a viewer permits. What bounds a hostile document today is the
address-space ceiling and the host's ability to kill the process. What is missing is a **cancel**,
which needs a host with a second thread — and that is a shape decision about this crate's API, not
a constant.

### 4. One rasterising thread, and the two candidate answers

ADR 0218 §2: `glibc`'s allocator sizes its arena count from `__get_nprocs()`, which reads
`/sys/devices/system/cpu/online`, so a thread's first allocation in a many-threaded confined
process is an `openat` the filter kills for. The two ways to get the cores back:

- **Per-thread Landlock plus an allocator warm-up.** Build the pool *before* seccomp with a
  `start_handler` that puts each worker in its own Landlock domain and allocates once, so the
  arena question is asked and answered while `openat` still returns `EACCES`. It rests on the
  allocator not asking again later, which is a claim about `glibc` internals — write it down as
  one, or measure it.
- **An allocator that does not consult the filesystem.** Every candidate this project has looked
  at contains `unsafe`, which is a separate decision.

Neither is free, and the thing that makes them worth doing is a measurement: one thread costs about
1 ms of the 7 ms this page takes.

### 5. The document crosses as bytes, and could cross as a descriptor

19.2 MB of ISO 32000-2 goes down a pipe once, and it is most of the 67 ms that document takes to
reach its first page. A `memfd` or an `SCM_RIGHTS` descriptor the confined side maps read-only
would remove the copy — and `pdf_syntax::Document` already takes bytes it does not own the
lifetime of. It needs `unsafe` for the mapping or a crate that has it, so it is a decision rather
than an optimisation.

## Two things that stayed true and are worth keeping

- **`viewer-core` needed no change at all.** Rules 2, 3 and 4 — no filesystem, no clock, no threads
  it was not handed — describe a confined process exactly, which is why this cost a transport
  rather than a redesign.
- **The sandbox is a flag and the default is the safe one.** There is no path in `viewer-confined`
  that interprets a document in the calling process when the worker will not start.
