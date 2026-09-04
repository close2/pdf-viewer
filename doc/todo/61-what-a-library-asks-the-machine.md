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

## The rule

**A probe is answered before the lockdown; it never becomes a permission.** The moment an
environment probe is allowed to widen the policy, every future crash has a cheap fix that costs the
boundary a little, and the boundary is the product. [59](59-the-resource-port.md)'s port exists for
resources the document names; nothing in this table is that, except the font lookup, which is in
that item precisely because it is the exception.

## What is owed

1. **Find the fifth instance before it finds us — the instrument exists and runs.** Session 917's
   ten classes are `crates/pdf-vfs/tests/read_corpus.rs`'s population since session 919 (ADR 0878),
   over every corpus root on the disk, with the fix removed as its own calibration. What is owed is
   not another instrument: it is that a round taking a *new* dependency into either confined worker
   runs the two sweeps below and says what they printed.
2. **The same sweep through `pdf-view-worker` — done in session 919** (ADR 0879).
   `crates/viewer-confined/tests/awkward_classes.rs` opens and draws the same population through
   the confined viewer, which is the program whose death costs a person the page they were reading
   rather than one generated file. `doc/verify.md` has the line; it is not a `doc/todo/02` §2 gate,
   because the read walk gates the same class of defect every round and this is the run a round
   touching the confinement owes.
3. A standing note wherever a new dependency is weighed (`doc/stack.md`): a crate that probes its
   environment costs a confined worker, and the cost is paid at the spawn or not at all.
