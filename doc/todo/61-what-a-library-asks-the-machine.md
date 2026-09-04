# 61 — What a library asks the machine, and why it is never a capability

Status: **accepted** by the project owner on 2026-09-04, as the line drawn through
[59](59-the-resource-port.md): the environment probes and the document's resources are two
different things, and only one of them is a capability.
Priority: 50-band. Companions: [59](59-the-resource-port.md), `doc/todo/34`, `doc/todo/58` §4,
traps 31 and its neighbours.

## The class

Four sessions have now met a confined worker dying on a syscall **no document caused**:

| session | what asked | what it read | how it was fixed |
|---|---|---|---|
| 902 | `available_parallelism` in the render path | `/proc/self/cgroup` | the answer taken before confinement (`RenderPlan::strips`) |
| 911 | glibc sizing a per-thread arena | `/sys/devices/system/cpu/online` | `MALLOC_ARENA_MAX=1` at the spawn |
| 914 | `pdf_font::substitute` looking for a face | `/usr/share/fonts` | `no_machine_fonts()` before the lockdown — **and this one *is* [59](59-the-resource-port.md)'s demand, not this item's** |
| 917 | the same as 914, reached by a different question | the same | round 914's fix, no second fix written |
| 920 | `std::os::fd::OwnedFd::drop`, closing a descriptor the worker was **handed** | nothing — it asks `fcntl(fd, F_GETFD)` first, under `core::ub_checks::check_library_ub()`, to catch a double close | the resource port sends bytes instead of a descriptor (ADR 0880); **the document's own descriptor is not fixed** |

## The rule

**A probe is answered before the lockdown; it never becomes a permission.** The moment an
environment probe is allowed to widen the policy, every future crash has a cheap fix that costs the
boundary a little, and the boundary is the product. [59](59-the-resource-port.md)'s port exists for
resources the document names; nothing in this table is that, except the font lookup, which is in
that item precisely because it is the exception.

## The fifth instance arrived, and it is not a probe

Session 920 found it by building `doc/todo/59`'s port, and it is worth recording as its own shape
because the rule above does not quite catch it. **It is not the environment being probed at all**:
nothing asks the machine anything, no library is sizing itself, and no filesystem call appears in the
code. It is the standard library checking a *precondition* on a descriptor the worker legitimately
holds, with a system call that is not on the allow-list, at `Drop`. Trap 32 has the trace.

Two consequences, and the second is owed work rather than a lesson:

- **The rule survives and is strengthened.** The fix was again not a permission: the port sends the
  resource's bytes rather than a descriptor the worker would have to drop. Widening the allow-list by
  one `fcntl` would have been a two-line change and is exactly what this item exists to refuse.
- **The document's descriptor is the open instance.** ADR 0812 hands the worker a descriptor per open
  document, and closing a document drops it. Nothing in this tree's suites closes a document in a
  confined worker, so nothing has met it; a `--release` build survives it and every debug build would
  not. **This is owed**, and it is the reason `doc/todo/61`'s first item is worth doing rather than a
  formality: a class sweep that opened and closed a document would have found it four sessions ago.

## What is owed

1. **Find the fifth instance before it finds us.** Session 917's `awkward_classes.rs` is the
   instrument: ten document classes through the confined transport, with the fix removed as its own
   calibration. It should be widened past `doc/pdf.js` and reconciled with `read_corpus.rs`, as
   `doc/todo/58` §4 records, rather than kept beside it.
2. **The same sweep through `pdf-view-worker`**, which has ADR 0870's probe and has never had the
   class sweep — session 914's finding that the viewer loses pages rather than glyphs came from
   reading, not from measuring.
3. **Close a document in a confined worker, in a debug build, and see what happens** — the paragraph
   above says what is expected and nothing has run it. It is one test in `viewer-confined`'s own
   suite, calibrated the way ADR 0870's two probes are.
4. A standing note wherever a new dependency is weighed (`doc/stack.md`): a crate that probes its
   environment costs a confined worker, and the cost is paid at the spawn or not at all.
