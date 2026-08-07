# ADR 0218 — A page drawn where it cannot be read from

Status: accepted, session 381.

## What was decided

**The document, the content interpreter and the rasteriser now run in a confined process**, in
the new crate `viewer-confined` and its program `pdf-view-worker`: seccomp-BPF, Landlock and a
4 GiB address-space ceiling, no filesystem and no network, with `viewer_core::Command` going in
and `Event` — and **pixels** — coming back. `CLAUDE.md` principle 3 asked for it and had been
answered only for the three image codecs (ADR 0014), which are by far the smaller surface.

Nothing in `viewer-core` changed, and nothing in `viewer-ui` uses it. The confined path is
reached today by `viewer_confined::Confined`, by its tests, and by
`examples/confined_page`; the window still interprets in process. Why that is a decision rather
than an unfinished job is §5.

## Why it cost one protocol rather than two

`doc/HANDOVER.md`'s section 0 recorded the open question as "the protocol would have to carry a
display list rather than an image, which is a real design question". It dissolves, and the reason
is the shape `viewer-core` already had:

> host toolkit ──Command──▶ viewer-core (no threads, no I/O, no clock) ──Event──▶ host

**A confined process is exactly the environment those three rules describe.** Rule 2 says the
host supplies bytes and the core produces bytes; rule 3 says there is no clock; rule 4 says there
are no threads the core was not handed. A crate written to those rules needs nothing from a
filesystem, which is why confining it required no change to it at all — the boundary was designed
for a *toolkit* and turns out to be a boundary for a *kernel*.

So the confined process is a **host of `viewer-core`**, and this crate's caller is a host of
pixels. The display list never leaves: `Event::NeedsRender` is answered inside, by `render-cpu`,
and `Command::RenderReady` is fed back into the same viewer. Those two messages are the only ones
that do not cross, and each is refused **by name** rather than dropped.

`Command::Save`'s own documentation predicted the arrangement before there was a process to prove
it — the host writes the bytes, "which is also what lets a confined process with none still
produce a saved file". Extraction, the password prompt and §12.7.6.4's import are the same shape,
and all three already crossed as messages.

## What is carried, and what is refused by name

Every `Command` crosses but `RenderReady`. Every `Event` crosses but `NeedsRender`. **Fourteen of
`viewer-core`'s twenty-five questions cross**; the other eleven — the outline, the layers, the
attachments, the collection, the articles, a thumbnail, the properties, Table 29's opening pair,
Table 147's preferences, the popups and §14.7's accessibility tree — answer with `pdf-model`
types, and encoding those is the second half of this boundary rather than a hole in the first.

Two properties hold that apart from a boundary that is quietly wrong:

- **The compiler names the variant nobody handled.** Every `match` in `protocol.rs` is exhaustive
  over a `viewer-core` enum, and nothing in that crate is `#[non_exhaustive]` — so a message added
  there fails to compile here. That is the same reason section 0 gives for keeping those enums
  exhaustive, one layer down.
- **A refusal is a message.** `Uncarried` carries the variant's name and the reason; the worker
  answers a question it cannot encode with a refusal *frame* and stays alive; a truncated or
  unrecognised message is a `ProtocolError` naming the field it died on. Nothing defaults, clamps
  or guesses, and no length from the confined side is allocated from before it is checked against
  the bytes that arrived.

### The sweep, run over the boundary it created

`doc/todo/01`'s fifth sweep asks whether anything one side can do the other cannot ask for.
Applied here — every variant of `Command`, `Event`, `Query` and `Answer` parsed out of
`viewer-core` and grepped against `protocol.rs` — the answer is **none**: 21 commands, 15 events,
25 questions and 26 answers, and every one of the 87 is named in the transport, either encoded or
refused with a reason. The exhaustive matches make that true at compile time from now on; the
sweep is what says it is true today.

## The three things the confinement forced, each measured rather than assumed

### 1. Images are decoded inside, because a confined process cannot spawn one

`pdf-model` decodes JBIG2, JPEG 2000 and CCITT through `pdf-sandbox`, which **starts another
process**. The interpreter's filter has no `execve` and no `fork` — deliberately, and
`tests/confined.rs` asserts it by trying — so the confined viewer sets
`pdf_sandbox::Isolation::InProcess` before it locks itself down.

This is not the in-process fallback `pdf-sandbox` refuses to have. The decoders run inside a
process confined at least as tightly as their own worker: same seccomp filter, same Landlock
domain, a *larger* address-space ceiling. What is lost is `pdf-sandbox`'s second reason —
panic containment — and the loss is one step rather than total: a decoder panic costs the confined
viewer rather than one image, so it costs **the page instead of the host**. `issue12963.pdf`, a
JBIG2 document, opens and draws through this path.

### 2. One rasterising thread, because `glibc` asks the kernel how many processors there are

The interesting finding of the round, and it took `strace -k` to see. A confined worker died of
`SIGSYS` on a page it had otherwise drawn, in the twenty-fourth `rayon` thread, at

```
openat(AT_FDCWD, "/sys/devices/system/cpu/online", O_RDONLY|O_CLOEXEC)
 > /usr/lib/libc.so.6(get_nprocs+0x14)
```

— `glibc`'s allocator sizing its arena count on a thread's first allocation. `arena_get2` asks
`__get_nprocs()` only once `narenas > mp_.arena_test`, which is why it appears at the twenty-fourth
thread and not the second, and **a bound derived from another library's internal constant is not a
bound**. So the confined worker rasterises on one thread, which cannot reach it.

`std::thread::available_parallelism` is the same class of problem — it reads `/proc/self/cgroup` —
and is asked once, before the confinement, in the one place it can be. `render-cpu`'s
`plan_strips` asked it on *every* rasterise, inside `unwrap_or`; it now asks only where the caller
did not say, which is the one change this round made to a rendering crate. It moves no pixel: the
argument is evaluated later, not differently, and `pdf-viewer`'s release binary is **byte-identical
before and after** (`md5 67e971517090cb680cc4164410c4f3cb` either way).

### 3. And that one thread found something ADR 0139 says is not there

ADR 0139's property is that "a machine with four cores and one with thirty-two draw the same
bytes", and `render-cpu`'s `strip_parallelism.rs` asserts it over six scenes. **On a real page it
is false by one pixel.** `doc/PDF20_AN001-BPC.pdf` page 1 at 500×700, drawn in one strip against
drawn in any number from two to thirty-two, differs at exactly one pixel — (117, 636), 127 against
111 in all three colour channels, alpha unmoved — and every division from 2 upwards agrees with
every other, so it is *one strip against more than one* rather than a question of where the cut
went.

The mechanism is not ADR 0138's re-parameterised curve, which is why the guard does not catch it:
a strip below the first is composed with `Transform::translate(0, -top)`, and a translated matrix
is a different `f32` rounding for a mark that lands on a coverage boundary. Trap 12b in one line —
six synthetic scenes test six synthetic scenes.

Two consequences, and only the first is this round's:

- `tests/confined.rs` pins the in-process side to one strip, so that its byte-for-byte comparison
  is a statement about the *confinement* and not about the strip planner.
- The claim itself is now a defect with a reproduction, in `doc/todo/12`.

## What it costs, measured

`examples/confined_page`, release, this machine, `doc/PDF20_AN001-BPC.pdf` at 900×1200
(849×1200 pixels drawn), three runs:

| step | confined | in this process |
|---|---|---|
| worker started **and confined** | 1.09, 1.10, 1.14 ms | — |
| opened, interpreted, page drawn | 6.7, 7.2, 8.7 ms | 6.0, 6.1, 6.4 ms |
| 4.1 MB of pixels across the pipe | 3.4, 3.8, 4.8 ms | none |

So a page costs roughly **twice** as long to reach a host through the boundary as it does in
process, and the pipe is the larger half of the difference — 4.1 MB at about 1 GB/s, which is the
order section 0's own estimate gave for a tier-1 host's memcpy. On `doc/ISO_32000-2_sponsored_EC3.pdf`,
19.2 MB, opening and drawing page one costs 66.9 and 82.6 ms confined, most of it the document
crossing the pipe once.

**The launch path is untouched and the measurement says so twice.** `--trace` under `Xvfb`, five
runs after the round: first present 108.0, 109.6, 110.9, 114.1, 122.4 ms, against 102.6, 102.8 and
109.7 before it on a machine that had not been building for an hour. The stronger statement is the
one that needs no clock: the `pdf-viewer` binary is byte-identical with and without this round's
changes, so *nothing* on the launch path moved.

## Why the window does not use it yet

Because putting it there is a decision with a number attached, and this round's job was to
produce the number rather than to spend it. Three things would have to be settled first, and each
is a real question:

- **The viewer is a tier-2 host.** It hands `Rendered::Presented` back and draws on the GPU; a
  confined process draws on the processor and hands over a raster. Going through the boundary
  means going to tier 1, which is a different first frame, not a slower one.
- **`CLAUDE.md` says page one goes to the graphics device**, by the owner's decision. A confined
  viewer as built draws page one on a single processor thread.
- **The panels would go dark.** Eleven queries do not cross, and four of the sidebar's six tabs
  are among them.

None of that is an argument against the boundary; it is an argument about *where* it goes, and
`doc/todo/34` now holds it.

## What was considered and rejected

- **Permitting `openat` and letting Landlock deny it.** It would make many-threaded rasterisation
  work at once, because `glibc`'s question would be answered `EACCES` rather than with a signal.
  Rejected: `pdf-sandbox`'s whole argument is that "seccomp carries the no-filesystem, no-network
  property on its own, and Landlock is depth", and this would move the property onto the depth
  layer — on a kernel booted without Landlock the confined interpreter could open files. Two
  seccomp filters cannot express it either: where filters disagree the kernel takes the strictest
  action, so a `KILL` in one is not softened by an `ERRNO` in another.
- **Warming the thread pool before the confinement.** `landlock_restrict_self` binds the calling
  thread and its future children, so threads made first would be outside the domain — the weaker
  of the two arrangements, and one `pdf-sandbox`'s own module header states as a rule.
- **Setting `RAYON_NUM_THREADS` or `mallopt(M_ARENA_MAX)`.** Both are `unsafe` in edition 2024 or
  through `libc`, and this crate is `#![forbid(unsafe_code)]`.
- **A deadline on the host's read.** A decode has a budget because one image's cost is bounded by
  its own dimensions; a page's is bounded by the document *and* the magnification, so a fixed
  number would refuse work the viewer permits. What bounds a hostile document here is the
  address-space ceiling and the host's ability to kill the process; what is missing is a cancel,
  and a cancel needs a host with a second thread. `doc/todo/34`.

## What this does not weaken

`pdf-sandbox`'s decoder profile is unchanged, syscall for syscall: `lockdown::apply` is
`apply_for(Profile::Decoder)` and `Profile::Interpreter` is a second list, four entries longer,
each entry named and measured. There is no path in `viewer-confined` that interprets a document in
the calling process when the worker will not start — `Confined::start` returns
`ConfinedError::WorkerDied` and the caller has nothing.
