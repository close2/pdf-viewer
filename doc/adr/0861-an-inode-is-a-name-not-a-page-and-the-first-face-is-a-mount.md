# 0861 — An inode is a name, not a page, and the first face is a mount

Session 909. Status: **accepted**. The second of this round's two records: RFC 0003 §7's FUSE
face, the dependency it took, and the three decisions a kernel forces on a design that was written
for a core.

## Context

RFC 0003 §7 recommends the order of construction and the reason for it: "core + FUSE face first
(pure Rust, testable in this tree's own harnesses, no external toolchain), KIO shim second (thin by
then, and its layout is already fixed by the core)." Three rounds built the core — the layout and
the read side (899), the confined worker (902), the write side and its transaction (906) — and
round 906's own record says what was left: "the FUSE face, which is the pure-Rust one and which now
has nothing left to discover about the core — `create`, `write_at`, `flush` and `release` are the
kernel's own verbs and `VfsError::errno` is a method."

## Decision

### 1. `fuser`, pinned to `=0.18.0`

RFC §7 names the crate and the hazard in one breath: "on `fuser`'s default pure-Rust `/dev/fuse`
path — no C linkage at all. Toolchain risk: **low**. (Note: fuser's 0.17 typed-API rework is
recent; pin and vendor-watch.)" The pin is `=`, not a caret, so a bump is a decision somebody
makes rather than a resolution somebody gets. The rework is exactly what this face is written
against: `INodeNo`, `FileHandle`, `OpenFlags` and `Errno` are types rather than integers, and
`Filesystem`'s methods take `&self`.

`doc/stack.md`'s rules, answered rather than asserted:

- **No C linkage, checked in the crate's own build script rather than believed.** `default = []`,
  and on Linux with no `libfuse` feature the script takes the `pure-rust` branch and never calls
  `pkg_config` — so no libfuse header, no `-l` flag, and a machine without `libfuse-dev` builds
  it. `libfuse`, `libfuse2`, `libfuse3` and `serializable` are off, and so is `experimental`,
  which would pull `tokio` into a program `CLAUDE.md` principle 2 forbids an async runtime in.
- **Four new locked packages**, counted from `Cargo.lock` rather than predicted: `fuser`, `nix`,
  `ref-cast` and its proc macro. Every other dependency it names — `libc`, `log`, `memchr`,
  `page_size`, `smallvec`, `zerocopy`, `bitflags`, `num_enum`, `parking_lot`, and `pkg-config` as
  a build script that does not run here — was already in this tree's lockfile. (This paragraph is
  itself the lesson `doc/stack.md`'s last row states: a package count is a prediction until it is
  measured, and this one was three before it was four.)
- **MIT, and MSRV 1.85** against this workspace's 1.97.
- **`#![forbid(unsafe_code)]` is unchanged** over this project's source. `nix` and `libc` carry
  `unsafe`, which is the same shape as `sha2`, `cmov` and `curve25519-dalek` already on
  `pdf-model`'s path, and is written down beside the dependency rather than left to be discovered.

### 2. An inode is a name, not a page

This is the one thing a FUSE face has to invent, and it is forced rather than chosen. RFC §5.2
makes an ordinal a position — "[o]rdinal names are **positions, not identities** — the rule that
makes insertion and deletion coherent: after any write, the next listing renumbers" — so there is
no page-shaped thing for an inode to be the identity *of*. `pages/0004.pdf` names the fourth page
of whatever the document now is.

So `Inodes` maps **path** to inode: one number per path, allocated on first sight, never reused,
kept for the life of the mount. Three consequences, each deliberate:

- **The attribute and entry timeouts are zero.** The kernel is told to ask again every time,
  because the answer can change for a reason no `stat` of ours would show — another program's
  incremental update, or our own commit. The cost is a round trip; the core's cache is what makes
  that cheap.
- **The `lookup` generation is always 0**, and that is correct rather than lazy: the kernel uses
  (inode, generation) to tell one file from another *after a number has been reused*, and this
  face never reuses one.
- **A path that stops existing keeps its number.** The kernel may still hold it, and handing the
  number to a different path later would make a stale `getattr` answer about somebody else.

### 3. The face holds no layout knowledge, and the mode bits are the proof

RFC §7: "The faces contain *no* layout knowledge — adding `fonts/` one day is a core change that
both faces grow simultaneously." There is no path pattern, no directory name and no generator in
`pdf-fuse`. The place that rule is easiest to break is `ls -l`, where a face wants to know which
files are writable — so the face asks `Vfs::write_meaning`, which is the core's layout table
answering, and a row the core does not name at all is read-only.

What the face *does* add is what a kernel needs and a core does not: the inode table above, a file
handle per open, and a number for every refusal. `open` materialises a `pdf_vfs::Handle` and files
it under a handle number, which is how RFC §5.4's "no reader ever receives a splice of two
generations" reaches a mount: the bytes are the generation's, and a later generation has no path to
a handle already open.

### 4. Every refusal is said as well as numbered

RFC §5.3 states the poverty and the remedy together: FUSE "returns `EPERM` with no message channel
— which is FUSE's poverty, and why the mount also logs each refusal's sentence to its own
stderr/journal". So `Face` takes a sink and every refusing method hands it the sentence before it
returns the number. That is trap 5 applied to a mount, and it is what turned up the missing
§7.5.6 sentence on an attachment deletion (ADR 0860 §3): the test that asserts the line found
nothing to assert on.

### 5. The invalidation task polls the generation key, and does not watch the file

RFC §5.4 requires the notifications to come "from a separate task — separate because issuing them
synchronously from a request handler can deadlock against the kernel (documented libfuse hazard)",
and names inotify as the mechanism. The thread is in the binary, which is the requirement; the
mechanism is a **poll of the generation key** every second, which is a departure and is recorded
as one.

The argument: the key is what an inotify event would send us to ask anyway — "(mtime, size, last
`startxref` offset)" — and the core validates it before every answer regardless. A watch would be
a second dependency and a second thing that can be wrong about one question. What the poll costs
is up to a second of staleness in a *file manager's cached listing*, never in an answer. What it
buys is that a file the mount is not looking at is not watched.

One notification *set* per change rather than one per name changed, because a change of the key can
move any name in the tree.

### 6. There is no switch that turns the confinement off

The binary builds its `Vfs` on `ConfinedWorkers` and offers no alternative. RFC §6's diagram is
the reason — "two thin, privileged FRONTENDS: file I/O, caching, verb mapping — and NOT ONE BYTE
of PDF parsing" — and a mount is fed hostile bytes by anything that opens a folder, which is what
makes it "the most exposed surface this project would ship". A `--in-process` flag would be a hole
in the one place the RFC calls load-bearing.

`--foreground` is accepted and is what the program always does; there is no fork, because a daemon
that detached would have nowhere to put §5.3's sentences. An option this program does not have is
refused by name rather than ignored, because an ignored `--read-only` is a mount somebody believes
is read-only.

## Consequences, and what is not tested

**A mount in a gate is a different question from a mount by hand**, and this round did the first
of the two only. `fuser`'s pure-Rust path asks the kernel for `/dev/fuse` and runs `fusermount3` to
attach it, so a gate that mounted would be measuring the machine's kernel configuration, its `fuse`
group membership and `/etc/fuse.conf` — none of which is a property of this tree, and any of which
turns a gate into a coin toss. That is `viewer-ffi`'s C-compiler argument, and it lands the same
way here.

So `crates/pdf-fuse/tests/a_face.rs` drives the face the way a kernel drives it, with no kernel:
the inode table, `lookup`/`getattr`/`readdir` over §4's tree, `open`/`read` and the generation an
open file keeps, `create`/`write`/`flush`/`release` with §7.5.6's prefix property read off the file
afterwards, a write released without a flush, a page inserted and a page deleted, and every §5.3
refusal as its `errno` *and* its sentence. What cannot be reached without a channel is `fuser`'s
wire format — the reply objects need a session — and the one part of that which is checkable
anyway is checked: `kernel.rs`'s unit tests hold the whole `pdf_vfs::Errno` → `fuser::Errno` table
against the numbers the core itself states, and against each other, so a mapping that wired
`EACCES` to `EPERM` fails.

What the face still owes is in `doc/todo/58`: a mount driven by hand and written up, the KIO shim,
and the three things `doc/todo/58` §4 says a face has to decide and this one decided by default —
what to do about a worker that died mid-operation, whether to pool workers, and a `Canceller` a
`Vfs` does not expose.
