# 0864 — A kernel asks questions no test thought of

Session 911. Status: **accepted**. The first of this round's two records: what happened when
`crates/pdf-fuse` was mounted by hand for the first time, and the seven defects the kernel found
in an hour that four rounds of tests had not.

## Context

`doc/todo/58` §3 carried one owed item against the FUSE face and stated it precisely:

> **nothing has ever mounted it.** `crates/pdf-fuse/tests/a_face.rs` drives the face the way a
> kernel drives it and holds everything between the kernel's verb and the core's answer, and a
> *mount* is deliberately not in any gate … **What is owed is a mount by hand, written up.**

That argument is still right and nothing here changes it: a gate that mounted would measure the
machine's kernel configuration, its `fuse` group and `/etc/fuse.conf`, which are not properties of
this tree (ADR 0861 §4). What it *also* means is that between the kernel's verb and the face's
method there was a whole vocabulary nobody had exercised — which flags an `open` carries, which
`setattr` a truncation is, what `cp` actually does to a name that already exists.

**It mounted on the first try.** `/dev/fuse` is world read-write here, `fusermount3` is setuid, the
mount appeared as `pdffs on … type fuse.pdf (rw,nosuid,nodev,noexec,noatime,user_id=1001,…)`, and
`ls`, `stat`, `find`, `du`, `tree`, `cat`, `file`, `qpdf --check`, `grep -r` and `cp` all worked
over RFC 0003 §4's tree without a single change to this tree. Everything below is what was wrong
underneath that.

## Decision

### 1. `open` ignores its flags, so RFC 0003 §5.2's first write verb was unreachable

The RFC leads with `cp new.pdf pages/0004.pdf` — "inserts before the current fourth page". In any
document with four pages, **that name already exists**, so `cp(1)` never issues `create` for it.
`strace` says what it issues:

```
openat(AT_FDCWD, ".../mnt/pages/0002.pdf", O_WRONLY|O_TRUNC) = 4
write(4, "%PDF-1.7\n%\277\367\242\376\n1 0 obj\n<< /Lang "..., 41453) = -1 EPERM
```

`Face::open` took an inode and nothing else and always materialised a **read** handle, so every
byte was refused. The verb the whole RFC is built around could be done through `create` — a name
past the end of the document — and through no other route: not `cp`, not `install`, not `dd`, not
a shell redirect, not a file manager's drop.

`Face::open` now takes the access mode, and a write-intent open stages a write exactly as `create`
does. Two things follow and both are improvements in their own right:

- **Refusals move to where a program looks for them.** `> mnt/text/0001.txt` used to open
  successfully and fail at the first byte — `echo: write error: Operation not permitted` — and now
  fails at the `open`, with §5.3's own sentence in the log, which is what a *new* name in the same
  directory had always done. The `errno` a shell reports also stops depending on whether the name
  exists: a render was `EPERM` when it existed and `EROFS` when it did not, and is `EROFS` both
  ways now.
- **A staged write starts empty whatever the name holds**, which is what `O_TRUNC` asks for and
  what all five verbs mean: the bytes copied in are the document to insert, the file to embed or
  the information dictionary to set, never an edit of what was generated.

### 2. `O_TRUNC` is a `setattr` with no file handle, and it was accepted and ignored

`fuser` does not negotiate `FUSE_ATOMIC_O_TRUNC`, so the kernel does the truncation itself: a
`SETATTR` of size zero between the `open` and the first `write`, carrying **no `fh`**. The face
handled only the `fh` form — `if let (Some(size), Some(handle))` — so the kernel's own truncation
fell through the `if` and the handler answered success. `: > mnt/pages/0002.pdf` then exited 0,
changed nothing and said nothing anywhere, which is precisely the lie `setattr`'s own doc comment
refuses `chmod` for. `Face::truncate_at` resolves it against the writes in flight by path.

### 3. A `close(2)` of a file nothing was written to is not a write

`touch(1)` opens `O_WRONLY|O_CREAT`, sets the times, and closes without a byte. With §1 in place
that became a commit of zero bytes, and `touch mnt/pages/0001.pdf` failed with *"not a PDF: no
%PDF- header in the first 0 bytes"* — an input/output error about a document nobody had touched.
So a staged write records whether anything ever wrote to it **or truncated it**, and a `flush` of
one that neither happened to commits nothing and says nothing. `: > file` still commits — a
truncation is an act — and still fails, loudly, because emptiness is not one of the five verbs.

### 4. `touch` succeeded and changed nothing, and the comment above the guard said it could not

Trap 28 at the smallest scale it comes in. `setattr`'s doc comment reads "[e]verything else —
mode, owner, times — is refused rather than accepted and ignored, because a `chmod` that returns
success and changes nothing is a lie a file manager will act on", and the guard beneath it tested
`mode`, `uid` and `gid`. Times are now in the guard, and `touch` says *"setting times of …:
Operation not permitted"*, which is true.

### 5. Every file was owned by `root`, and every one was dated 1 January 1970

Two separate wrongs with one shape: an answer nobody had had a reason to look at.

- **Owner.** `attributes` wrote `uid: 0, gid: 0` on a mount whose own `user_id` is 1001. Nothing
  was *permitted* that should not have been — `default_permissions` is not among the mount options
  and the session's access-control list is the mounting user alone — but every program that reads
  a listing rather than trying the operation was told the wrong thing, and a file manager greys
  out what it believes belongs to root. It is now the real user and group of the process, read
  from `/proc/self`'s own ownership, which is where they are without a system call a crate under
  `#![forbid(unsafe_code)]` can make.
- **Times.** The epoch, and the doc comment argued for it: "inventing 'now' would make every `ls`
  of a mount look like a directory that had just changed". Right about `now`, wrong about the
  alternative, which the same comment names — "[a] face that wanted the backing file's times would
  take them from the generation key, which is the only clock this design has". RFC 0003 §5.4's key
  *is* `(mtime, size, last startxref offset)`, so the answer was already in the tree. Every name
  now reports the document's own mtime, because every name is derived from the whole document.

### 6. An `ENOENT` is an absence, not a refusal, and logging it floods the only channel there is

RFC §5.3 makes the mount's own standard error the place a refusal's sentence goes, because FUSE
has none. The face logged one for every failed lookup as well — and a file manager probes
`.directory` in every directory it opens, macOS writes `.DS_Store`, and something on this machine
looked for `.gitignore`: **fifteen lines appeared before the mount had been asked to do anything
at all.** `ENOENT` is no longer logged; everything else still is.

### 7. Two writes in flight in one mount lost the second, quietly

A staged write whose generation has moved is `ESTALE`, which RFC §5.4 requires of *somebody else's*
edit: "committing it would discard whatever changed it". Our own commit is not that — §7.5.6
appends, so it discards nothing — and the check did not ask whose the edit was, although
`Provenance` exists for exactly that question and round 906 built it. Two files copied into
`attachments/` with both descriptors open lost the second, and lost it **quietly**, because
`close(2)`'s error is a thing most programs do not look at.

What decides now is a per-`Vfs` count of *foreign* transitions, recorded on the staged write, which
is exact across any number of generations where the served generation's own `Provenance` flag —
describing only the last transition — is not. And whose the edit is is only half of it: an
embedded file's name and the information dictionary are **identities**, so they still mean what
they meant; a page's ordinal is a **position** — §5.2, "after any write, the next listing
renumbers" — so an insertion staged across a commit that renumbered would land somewhere nobody
asked for, and stays `ESTALE`. A rebased write re-asks the layout, so every check `create` made is
made again against the generation it will actually be committed to.

## Consequences

`crates/pdf-fuse/tests/a_face.rs` gains three tests and `crates/pdf-vfs/tests/a_write.rs` one, and
each is a defect above rather than a demonstration of one. What no gate can still reach is what
this record is about: `fuser`'s wire format, and the vocabulary a kernel speaks through it.
`doc/todo/58` §3 now carries the *next* mount by hand rather than the first, and the commands are
in `doc/history/911-*.md` so that it is a repetition rather than a rediscovery.

Three questions `doc/todo/58` §3 asked, answered by the mount:

- **`readdir`'s offsets survive a directory larger than one reply buffer.** 1023 names came back
  once each, in order, with no duplicate and no gap.
- **A zero attribute timeout is not what a stat storm costs.** §5.5's rule that a `stat` generates
  is — see ADR 0865 §3.
- **`--allow-other` cannot be used on this machine, and says so**: `fusermount3: option
  allow_other only allowed if 'user_allow_other' is set in /etc/fuse.conf`, which is commented out
  here.

And one behaviour that is not a defect and had better be written down: **`mv mnt/pages/0003.pdf
~/` deletes the page from the document.** `mv` across file systems is a copy and then an `unlink`,
and an `unlink` in `pages/` is §5.2's deletion verb; §7.5.6's "the bytes stay in the file" sentence
is logged, as it is for any deletion. That is the archive-manager convention and it is the correct
composition of two verbs this face has — but a person dragging a page out of a file manager
window will not have read either of them.
