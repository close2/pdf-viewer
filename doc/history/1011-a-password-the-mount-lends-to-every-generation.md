# 1011 — A password the mount lends to every generation

Session 1011, general improvement, on `batch-1006-1011` with five siblings live in the worktree.
Date: 2026-09-12/13. ADR: [1030](../adr/1030-a-password-the-mount-lends-to-every-generation.md).

**The finding.** Every consumer of a document in this tree could be given ISO 32000-2 §7.6.4.1's
password except the one that opens documents on behalf of the whole machine: `pdf-vfs`'s mount
passed a literal `None` at the one place it spawns a worker, so `pdffs doc.pdf mnt/` refused every
encrypted document the clause's default user password does not open. The plumbing for one existed
all the way to the wire and stopped one call short of the top — `doc/todo/02` §1's fourth shape,
a capability that reached the crate and never reached the program — and both of this crate's corpus
walks had grown a worker factory of their own to get around it.

**What changed.** `Workers::spawn` lends the password (`Option<&Secret>`) instead of taking it, so
the mount can hold one for its life and hand it to every generation's worker without the `Clone`
that `viewer_core::Secret` refuses; `Vfs::with_password` is where a face supplies one; `pdffs` gains
`--password-fd <n>` on the transform suite's own convention, with `--password` refused by name
because argv is public. `Vfs::shortfalls`'s encryption entry is rewritten to what is still true —
a password cannot be supplied *after* the mount exists — and the assertion that held it stopped
matching on a substring the new sentence would also have contained.

**What the round also did was eliminate**, with numbers, three things it was pointed at: all 27
`Command` and all 32 `Query` variants reach a host; two unused `pub fn`s in fourteen crates, one of
them a defensible conversion pair; and `raster-function-conformance`'s cases are run on the device
by `raster-gpu`, so it is not marking its own homework.

**Files touched.** `crates/pdf-vfs/src/{lib,worker,confined}.rs`, `crates/pdf-fuse/src/main.rs`,
`crates/pdf-vfs/tests/{a_face,a_write,confined,read_corpus,write_corpus}.rs`,
`crates/pdf-vfs/examples/faces_on_the_port.rs`, `doc/todo/58-the-file-system-faces.md`,
`doc/adr/1030-…md` and this file.

**Left undone and named**: the KIO face still cannot ask, because `quorra_vfs_mount_open` has no
password parameter — one C entry point and an ABI version bump, now written into `doc/todo/58` §5.
