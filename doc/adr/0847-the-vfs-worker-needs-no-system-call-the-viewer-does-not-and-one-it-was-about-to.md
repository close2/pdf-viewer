# 0847 — The vfs worker needs no system call the viewer does not, and one it was about to

Session 902. Status: **accepted**. The second of this round's two records: RFC 0003 §6's confined
generator, the allow-list it runs under, and the defect that allow-list found.

## Context

`doc/todo/58` §4: "**No face ships before it exists**: a mount is entered by anything that touches
a folder, and a file manager will open a document nobody chose to open." `pdf_vfs::InProcessWorkers`
parses hostile bytes with the caller's privileges, so RFC 0003 §6's diagram — two thin privileged
frontends over one core over one confined worker — has been a drawing since round 899.

The seam was built to make this a transport change (ADR 0841 §2), and it was: `Query` and `Answer`
are plain data, a worker is made once per generation, and the document is handed over at exactly
the moment ADR 0812's `SCM_RIGHTS` route wants it. What remained to decide is the thing a
confinement actually *is* — the allow-list — and to prove it.

## Decision

### 1. `Profile::Interpreter`, unchanged. Not a third profile

`pdf_sandbox::lockdown::Profile`'s own documentation says a profile is "a property of the work,
found by running that work under `strace` and reading what appeared", and that "adding a third
means measuring a third". It was measured, and the answer is that there is no third to add: **the
vfs worker needs exactly the viewer's list and not one call more.**

The reason is structural rather than lucky — what this worker does is what `pdf-view-worker`
already does:

- It **parses** a document handed to it as a descriptor, through the same `pdf_syntax::FileBytes`:
  the same `recvmsg` for the descriptor and `pread64` for the bytes, and no `openat`, `statx`,
  `lseek` or `fstat`. The file's length crosses *in the open frame* rather than being asked for,
  which is the whole of why `statx` stays off.
- It **draws** pages with the same `render-cpu` on a `rayon` pool of one thread, so the same
  `clone3`, `clone`, `rseq`, `set_robust_list` and `sched_getaffinity`.
- It **decodes** §7.4.6's, §7.4.7's and §7.4.9's codecs in-process, because a confined process
  cannot spawn `pdf-sandbox-worker` — which is ADR 0218's decision for exactly this case.
- Everything it **writes** goes into `pdf_transform::MemorySinks`, which is memory. A transform's
  file sinks would need `openat` and this worker never constructs one; `RLIMIT_FSIZE` is 0 in the
  confinement, so even a sink that tried would be killed rather than obeyed.

Measured with `strace -ff` over `pdf-vfs-worker` answering every question in `Query` on
`doc/PDF20_AN001-BPC.pdf`. After the filter is installed, the worker's own thread issues
`pread64`, `recvmsg`, `write`, `brk`, `mmap`, `munmap`, `madvise`, `futex`, `rt_sigprocmask`,
`rt_sigaction`, `getrandom` and `clone3`; the one rayon thread it makes issues `sched_yield`,
`futex`, `sigaltstack`, `mmap`, `munmap`, `mprotect`, `set_robust_list`, `sched_getaffinity`,
`rseq`, `rt_sigprocmask` and `gettid`. Every one of those twenty-three is on `PERMITTED` or
`PERMITTED_INTERPRETER_EXTRA`. Nothing was added.

### 2. And the allow-list found a defect on the way

**`pdf_transform::render` drew with `CpuRasterizer::new()`, which asks the machine how many cores
it has** — and `std::thread::available_parallelism` reads `/proc/self/cgroup` on Linux, which is
an `openat` a confined process is *killed* for rather than told no (ADR 0218 §2). `render-cpu`'s
own `plan_strips` says so in a comment three lines long, and says what such a caller does instead:
"[s]uch a caller states the number with `with_strips`".

Nothing in the transform suite could state it. So `RenderPlan` gains `strips: Option<u32>`, `None`
everywhere the suite already was — a batch render is parallel across pages and the strips are the
rasteriser's judgement about one page — and `Some(1)` in the confined worker, taken before its
confinement.

It is worth being precise about what this was: not a latent risk, but a **kill, reproduced**. With
`strips: None` the round's own comparison test fails with

> the confined worker stopped without answering (killed by signal 31 (SIGSYS: a system call the
> confinement forbids))

on the 150 dpi render and on nothing before it. The confinement is what found it; a reviewer
reading the render path would not have.

### 3. What crosses, and what the bound is

RFC 0003 §5's reads produce bytes: a page's PDF, a PNG, text, an attachment's payload, JSON. All
of it crosses as **one frame, bounded, refused past the bound** — not streamed, and the reason is
the core's own shape rather than an aversion to streaming. RFC §5.5 makes a `stat` generate,
because "an under-estimate silently truncates a page", and `Handle` therefore holds the whole file
already; a wire that streamed would be streaming into a buffer the broker has to fill before it
can answer the `stat` that precedes every read.

The bound is `confined_transport::ceiling::message_budget`, and it is derived rather than chosen:
the 4 GiB ceiling `Profile::Interpreter` installs, less what the process already occupied before
the confinement, less ADR 0597's 128 MiB settling allowance, less `Budget::max_pixels × 4` for the
page's own raster, divided by the two copies of an answer that live at once — the sink's and the
frame's. On this machine, with the default budget's 2²⁸ pixels, that is **1 464 MiB**, and it is
re-derived when a broker opens a document with a `max_pixels` of its own.

**Both directions are checked, and the answer direction is the one that matters here**: a mount
will be asked for a 300 dpi render of a large page. A worker whose encoded answer is past the
budget refuses it *by name*, with a sentence saying what the ceiling is and why a message costs two
copies of itself, rather than writing a frame the broker cannot hold. The broker independently
refuses a frame it cannot reserve, reads past its bytes, and keeps the worker — because the worker
is the untrusted side and a length it states is a claim.

### 4. A death is an answer, and the next question gets a fresh worker

The worker is killable by design — by its own seccomp filter, by `RLIMIT_AS`, by a panic under
`panic = "abort"`. Every one of those closes its output, which is what makes the broker's blocking
read return, and it arrives as `WorkerError::Transport` carrying the worker's own last words.

What that alone would leave is worse than the death: a mount whose every later operation produced a
stranger error about a closed descriptor. So `Worker` gains `is_alive`, `Confined` sets it false on
any transport failure and never clears it, and **`Vfs::current` asks it beside the generation key**.
A dead worker throws the generation away exactly as a changed file does, so the operation after a
death starts a fresh worker over the same document. `InProcess` is always alive and pays an atomic
load.

### 5. One vocabulary of refusals, not two

`WorkerError::Refused` carried `pdf_transform::Refusal` — a dozen structured variants. Re-encoding
those on the wire would put a second copy of that vocabulary in `pdf-vfs`, and *not* encoding them
would give a face two different answers for one document depending on which worker replied. Both
are the thing the seam exists to prevent, so the variant now carries the sentence, beside one
distinction a face can act on: `PasswordRequired`, which means *ask somebody for something* rather
than *this cannot be done*. `WorkerError::of` is the single place the seam's population is
narrowed, so the two implementations cannot disagree.

## Consequences

- RFC 0003's faces are unblocked: `doc/todo/58` §4 is done, and a FUSE daemon or a KIO shim now
  chooses `ConfinedWorkers` and inherits seccomp, Landlock and a 4 GiB ceiling without knowing they
  exist.
- The confinement costs **a few milliseconds per generation** (the spawn, the filter, the
  ruleset, the greeting and the descriptor) and **tens of microseconds per question** on top of
  the work; on a 300 dpi render, which is where a mount's time actually goes, it is inside the
  run-to-run spread and sometimes the confined side is the faster of the two.
  `examples/vfs_cost` prints all of it, and this round's record has the run.
- Six probes hold the boundary rather than describing it: a confined generator cannot open a file,
  reach the network, start a program or stat a descriptor it holds; it *can* read that descriptor
  where the file's offsets point; and it can still derive every file the layout offers. A
  confinement with no test that it kills is a claim.
- `pdf-vfs-worker` is a separate binary, and trap 10 does *not* bite on the path this crate is
  tested by — checked rather than assumed: `cargo nextest run --workspace` and `cargo test -p
  pdf-vfs` both build a package's bin targets, so the worker beside the test binary is this
  build's. It would bite on a `--profile gates --test` line, which builds one test target and
  nothing else, and `doc/todo/02` §2's map now says so. Every test that starts a worker names the
  build command when it cannot find one.
