# 61 — What a library asks the machine, and why it is never a capability

Status: **accepted** by the project owner on 2026-09-04, as the line drawn through
[59](59-the-resource-port.md): the environment probes and the document's resources are two
different things, and only one of them is a capability.
Priority: 50-band. Companions: [59](59-the-resource-port.md), `doc/todo/34`, `doc/todo/58` §4,
traps 31 and 32.

## The class

Six sessions have now met a confined worker dying on a system call **no document caused**:

| session | what asked | what it read | how it was fixed |
|---|---|---|---|
| 902 | `available_parallelism` in the render path | `/proc/self/cgroup` | the answer taken before confinement (`RenderPlan::strips`) |
| 911 | glibc sizing a per-thread arena | `/sys/devices/system/cpu/online` | `MALLOC_ARENA_MAX=1` at the spawn |
| 914 | `pdf_font::substitute` looking for a face | `/usr/share/fonts` | `no_machine_fonts()` before the lockdown — **and this one *is* [59](59-the-resource-port.md)'s demand, not this item's** |
| 917 | the same as 914, reached by a different question | the same | round 914's fix, no second fix written |
| 920 | `std::os::fd::OwnedFd::drop`, closing a *face* the broker handed over | nothing — it asks `fcntl(fd, F_GETFD)` first, under `core::ub_checks::check_library_ub()`, to catch a double close | the resource port sends bytes instead of a descriptor (ADR 0880) |
| 924 | the same, closing the **document's** descriptor, which cannot cross any other way (ADR 0812) | the same | `fcntl` admitted for the interpreter profile alone, **narrowed by argument to `F_GETFD`** (ADR 0888) |

## The rule

**A probe is answered before the lockdown; it never becomes a permission.** The moment an
environment probe is allowed to widen the policy, every future crash has a cheap fix that costs the
boundary a little, and the boundary is the product. [59](59-the-resource-port.md)'s port exists for
resources the document names; nothing in this table is that, except the font lookup, which is in
that item precisely because it is the exception.

**And the table has two shapes in it, which is what sessions 920 and 924 established.** The rule
above is about the first; the second is not a probe at all and must not be treated as one, in
either direction — neither answered before the lockdown, which is impossible, nor waved through as
"the same kind of thing":

| shape | what the process is doing | what to do | rows |
|---|---|---|---|
| a probe | asking the **machine** about itself, so that a library can size itself | answer it before the lockdown | 902, 911, 914, 917 |
| a precondition | the standard library checking a resource the worker was **given**, at the moment it is given back | send the resource another way; where it cannot cross another way, **one command, narrowed by argument** | 920, 924 |

The second column of the second row has an order and both halves have been spent. Sending the
resource another way is the first answer and ADR 0880 §6 is where it worked — a face crosses as
bytes. ADR 0888 is what is left when the resource is a descriptor by construction, which the
document's is: ADR 0812 exists precisely so that a 6 GB file does *not* cross as bytes. A third
instance of this shape is a question about which of those two applies, never a reason to reach for
the second first.

## The precondition shape, and how the two instances of it were answered differently

Session 920 found it by building `doc/todo/59`'s port. **It is not the environment being probed at
all**: nothing asks the machine anything, no library is sizing itself, and no filesystem call
appears in the code. It is the standard library checking a *precondition* on a descriptor the worker
legitimately holds, with a system call that is not on the allow-list, at `Drop`. Trap 32 has the
trace.

- **A face crosses as bytes** (920, ADR 0880 §6). The fix was not a permission: the answer frame
  carries the resource rather than a descriptor the worker would have to drop, at the cost of one
  copy of a file that is tens of megabytes at worst, in the process that has the memory. This is the
  first answer to reach for and it is the one that leaves the boundary alone.
- **The document cannot** (924, ADR 0888). ADR 0812 hands the worker a descriptor per open document
  precisely so that a 6 GB file does not cross as bytes — the route it would have to take is the one
  that aborted the *host* at 10.44 GiB. So the second answer was spent here: `fcntl` on the
  interpreter profile, **narrowed by argument** to `F_GETFD`, which reads the close-on-exec flag of a
  descriptor the process already holds and can open, create and change nothing.

  The two alternatives were priced and both are leaks with arithmetic behind them rather than
  aesthetics: `DESCRIPTOR_LIMIT` is 8, three are inherited, so a worker that never gives a descriptor
  back refuses every document after the fifth a person opens and closes — a user-visible defect in
  the *release* build, introduced to fix one only debug builds have. ADR 0888 §2–4 has all four
  candidates, including turning the library-UB checks off in the worker, which is the tempting one.

**What this cost the item, stated plainly**: the allow-list moved, for the first time since ADR 0812,
and the rule at the top of this file is what says that is not a precedent. A probe is still answered
before the lockdown and still never becomes a permission. The next round to reach for the second row
of that table owes the first column of it first — *can the resource cross another way?* — and 920 is
the evidence that the answer is usually yes.

## What is owed

1. **Find the next instance before it finds us — the instrument exists and runs.** Session 917's
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
3. **Close a document in a confined worker, in a debug build — done in session 924** (ADRs 0888,
   0889). `viewer-confined`'s
   `a_document_closed_in_the_confined_process_leaves_a_worker_that_still_answers` is the witness and
   is no longer `#[ignore]`d; it is a gate under `cargo nextest run --workspace`, which is the run
   whose worker has the library-UB check compiled in at all — `--profile gates` inherits `release`
   and issues the call nowhere. Three probes stand beside it, one per direction the rule can be
   widened in: the call is permitted, the *other commands* of the same call still kill, and the
   **decoder** profile still kills for all of them. Each was calibrated against the policy made
   wrong in that particular way (trap 13; ADR 0889 has the table).
4. A standing note wherever a new dependency is weighed (`doc/stack.md`): a crate that probes its
   environment costs a confined worker, and the cost is paid at the spawn or not at all.
