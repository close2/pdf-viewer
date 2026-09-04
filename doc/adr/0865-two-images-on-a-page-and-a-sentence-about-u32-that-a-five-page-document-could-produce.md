# 0865 — Two images on a page, and a sentence about `u32::MAX` a five-page document could produce

Session 911. Status: **accepted**. The second of this round's two records: three defects a mount
found *below* the face — one in the confinement, one in `pdf-transform`'s structure carry, one in
the core's cache — none of which is about FUSE at all.

## Context

ADR 0864 is what the kernel found. This is what came out of using the mount the way RFC 0003 §1
advertises it — "once a document is a directory, every tool the user already has becomes a PDF
tool" — on documents larger than the four the tests carry. Every one of these was reached by an
`ls`, and none of them has anything to do with a file system.

## Decision

### 1. A page with two images killed the confined worker

`ls mnt/images/0060/` on `doc/Tagged-PDF-Best-Practice-Guide.pdf` answered `EIO`, and the mount's
log said why:

> readdir: the confined worker stopped without answering (killed by signal 31 (SIGSYS: a system
> call the confinement forbids))

The kernel's own audit line names it: `sig=31 arch=c000003e syscall=257` — `openat`. Nothing in an
image extraction opens a file. What does is **`glibc`**, which creates a per-thread allocator arena
at a thread's *first* allocation and sizes the arena count from `__get_nprocs`, which reads
`/sys/devices/system/cpu/online`.

Page 60 is the only page of that document with **two** images, and that is the whole
discriminator: `pdf_transform::images` collects with `par_iter`, `rayon` runs a single item on the
calling thread — whose arena has existed since before the confinement — and hands two to the pool,
whose thread then allocates for the first time and dies.

This is ADR 0847 §2's finding one layer down, and its fix does not reach here. `pdf_vfs::serve`
already builds `rayon`'s pool with the thread count stated, precisely so that `rayon` does not ask
`available_parallelism`; the arena is a *different* `openat`, made by the allocator rather than by
the pool, and it happens on the pool's thread rather than on the one that confined itself.

**The fix is `MALLOC_ARENA_MAX=1`, set by whoever spawns**, in
`confined_transport::Host::start` — which is the one place early enough, because `glibc` reads the
variable at start-up and the worker is already past it by `main`. It covers `pdf-view-worker` as
well as `pdf-vfs-worker`, which is right: both are under the same profile and both run their
rasterisation on one thread by `pdf_vfs::serve`'s own finding.

**Why not move the pool in front of the confinement.** The seccomp filter is installed with
`TSYNC` and would still reach a thread made earlier, but the **Landlock domain is per-thread**, so
a pool built before `apply_for` would leave its worker outside it. `serve`'s doc comment already
says the pool is built after the confinement "so that its thread inherits both the Landlock domain
and the seccomp filter", and that stays true.

**What this cost before it was found**: nothing in `tests/confined.rs` asked a question that
dispatched, so all six probes passed while `ls` on a real document killed the worker. The new
probe is `a_page_with_two_images_does_not_kill_the_worker`, and it was run against the tree with
the environment line commented out before it was believed (trap 13): it fails there and passes
here.

### 2. `cp mnt/pages/0001.pdf` refused every page of ISO 32000-2, with a sentence that was false

`ls -l mnt/pages/` on the 1023-page ISO 32000-2 produced **1674 `EIO`s**, all of one sentence:

> the derived document needs more objects than one file can number

That sentence is about `u32::MAX`. The document has 33 000 objects. The sentence was reached from
`Host::replace_object`, whose signature was `Option<ObjectId>` and whose two implementations were
`self.assembly.replace(0, id).ok()` — so `AssemblyError::AlreadyPlaced` arrived as `None` and was
renamed into a claim about arithmetic. Principle 1: an error is propagated, not renamed. Both
methods now return the assembly's own error, and the true one is `object 16 was already placed`.

**The defect underneath.** `Carry::plan`'s own doc comment states the invariant: "[e]very kept
element is given its slot here, before any page is built, so that a reference to one from anywhere
in the closure maps to the rebuilt element rather than dragging the source's whole subtree in
behind it." `Carry::decide` runs *before* `Carry::number`, and `decide` carried the values of every
Table 357 / Table 358 content item through the host's closure walk. ISO 32000-2 writes a `/P`
back-reference on its object references —

```
16619 0 obj << /K [ << /Obj 1125 0 R /P 16619 0 R /Pg 44 0 R /Type /OBJR >> … ] /S /Link >>
```

— so carrying that item **copied element 16619**, and the element's own `replace` a moment later
found the slot taken. `/P` is Table 355's key on a structure element and is on neither content
item's table; but the fix is not to drop a key the producer wrote (RFC 0002 §11.1), it is to keep
the ordering the comment already promised. `Child::Reference` holds the source dictionary
uncarried beside the output number of its page, and `Carry::element` carries it after `number` has
run — at which point the closure maps a reference to a kept element onto its rebuilt slot, which
is what the invariant says should happen.

With that, all 1023 pages extract, `qpdf --check` is clean on the piece, and the structure tree is
carried (235 elements written, 78 233 dropped for reaching no page the output holds).

### 3. A second `ls -l` cost more than the first

Also on those 1023 pages: 2 min 45 s the first time and **4 min 03 s the second**. RFC 0003 §5.5
makes a `stat` generate — "an under-estimate silently truncates a page", the ffmpegfs lesson — and
each piece of that document is about 1.8 MB, so 1023 of them do not fit in any sane cache budget
and every entry had been evicted before the listing came round again.

**A length taken off the bytes themselves is not an estimate**, whether or not those bytes are
still held. The cache now keeps `(generation, path) → size` after the bytes are evicted, and
`Vfs::stat` answers from it. The notes are bounded by the document rather than by a number — one
per path per generation, dropped with everything else when the generation changes — and a note is a
path and eight bytes.

The gate for it needed an instrument the tree did not have, and that is worth the sentence: a size
is the same number whether it was remembered or recomputed, so the first version of the test passed
without the fix. `Vfs::generated` counts what was actually produced, and the test asserts the second
listing produced nothing. It then failed for real — 10 against 5 — because `Cache::put` returns
early for an entry larger than the whole budget, which is exactly the population whose `stat` is
expensive. The note is now taken before that return.

## Consequences

Three fixes in three crates, none of them in the face, all three found by `ls`. That is the
argument for a by-hand mount stated as a result rather than as a hope: RFC 0003 §1's claim is that
the transform layer gets "a second, involuntary user interface — which is the best possible test of
its API being honest", and the first hour of using it turned up a confinement kill on a real
document, a refusal whose sentence was false on every page of the standard itself, and a cache
whose second answer cost more than its first.

What is *not* fixed, and is now in `doc/todo/58` §5 with a number beside it: a `stat` still
generates the first time, so `ls -l` on a thousand-page document costs the whole extraction once —
2 min 45 s here — and the pieces are 1.8 MB each because a split carries the closure of a heavily
shared document. Neither is a defect; both are what a face would have to answer before a file
manager could open such a mount comfortably.
