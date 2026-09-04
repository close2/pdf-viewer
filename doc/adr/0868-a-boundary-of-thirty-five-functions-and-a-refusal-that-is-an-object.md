# 0868 — A boundary of thirty-five functions, and a refusal that is an object

Session 913. Status: **accepted**. The first of this round's two records: the C ABI RFC 0003 §7
requires between `pdf-vfs` and a face that cannot be written in Rust, and the four shapes it is
made of. ADR 0869 is the face itself.

## Context

RFC 0003 §7 states the constraint rather than choosing it:

> KF6 admits no Rust worker — no binding exists (the one experiment, cxx-kde-frameworks, was
> archived in 2024 at 19 commits), and `WorkerBase` is a C++ class. So: a deliberately dumb C++
> `MODULE` plugin subclassing `KIO::WorkerBase` … every operation forwards over a **C ABI** into
> the core — the `viewer-ffi` precedent, which is exactly the boundary this tree already knows how
> to freeze and test.

`viewer-ffi` is therefore not an analogy here, it is the instruction. What follows is that
precedent applied to a tree of files rather than to a window, and the four places the two
boundaries differ — because a difference taken without noticing would be this crate quietly
deciding something `pdf-vfs` had already decided.

## Decision

### 1. `crates/pdf-vfs-ffi`, one module of `unsafe`, thirty-five entry points

Same shape as `viewer-ffi`, deliberately and without variation: `#![deny(unsafe_code)]` with the
permission granted to `abi` alone, every entry point `pub unsafe extern "C" fn`, every body doing
three things and nothing else — raw arguments into safe Rust, one call into a safe module, the
result through an out-parameter. `tests/unsafe_position.rs` counts the tokens and asserts the
position: one `unsafe` per signature, three in the shared helpers', and the word absent from every
other file except as prose.

**A verb is a function rather than a tagged union**, for `viewer-ffi`'s reason and it is entirely
about C: a union's size is part of the ABI, so a verb added later changes the size of a type every
caller has already compiled, and an old caller passing an old-sized struct to a new library is
undefined behaviour no diagnostic catches. There is exactly **one** shape passed by value —
`pdfvfs_attributes`, three fields — and `PDFVFS_ABI_VERSION` exists for it and for nothing else.

### 2. A refusal is an **object**, not a return value and not a slot

This is the first of the four differences, and it is the one that decides whether this face is
worth building at all.

RFC 0003 §5.3 requires every refusal to carry a sentence, and §7 requires the number to be the
core's: "a KIO worker mapping these onto `KIO::Error` and a FUSE daemon handing them to the kernel
must agree about what a refused write *is*, and a face that chose its own numbers would be the
second copy of a decision." So both halves have to survive the call, and a C caller cannot see a
`Result`.

Three candidates. A **status enumeration wide enough to carry the errno** folds two populations
that must not be folded — a null pointer from the shim and an `EPERM` from the layout would be
told apart by arithmetic on one number, and `viewer-ffi`'s `status.rs` already argues why the
caller's own mistakes and the document's answers are different things. A **last-error slot on the
mount**, `errno(3)`'s own shape, is a global a caller may forget to read, is written by calls that
did not fail, and is overwritten by whichever of two threads got there second. An **owned object**
is written only where the status is `PDFVFS_REFUSED`, is read where the caller likes, and is freed
by the caller — `viewer-ffi`'s owned-batch discipline applied to one message.

The third. Every fallible entry point takes a `pdfvfs_refusal **why`, and the discipline is a
property of the code rather than of thirty-five separate arms: `abi::refused` is the only function
that writes it.

### 3. The counted enumeration is `errno`, and `pdfvfs_errno_name` answers for numbers it does not
know

`viewer-ffi`'s question, one boundary over: a C caller cannot be made to fail its build when a
kind is added, so the count is checkable at startup instead. `PDFVFS_ERRNO_KIND_COUNT` is what the
header states, `pdfvfs_errno_kind_count()` is what the library answers, and `pdfvfs_abi_check`
compares them with the version — "fails to compile in every consumer" becoming "fails to start,
once, saying which number moved".

What makes it a check rather than a restatement is in `refusal.rs`: `tag` is an exhaustive `match`
over `pdf_vfs::Errno` whose only purpose is that it **cannot be written incompletely**, so a
fourteenth kind added to the core stops this crate compiling; and a test walks every entry of
`KINDS` through it and requires every tag from zero to the count to appear. The C side's guarantee
is weaker than the Rust side's by exactly one step, and that step is now the only one.

`pdfvfs_errno_name` answers a name for **every** number, including one this build has never heard
of — trap 5 in the form C leaves available, and the same answer `viewer-ffi` gives an event kind it
does not know.

### 4. The staged four are **not** on this boundary, and that is the second difference

`create`, `write_at`, `flush` and `release` are what a kernel hands a file system one call at a
time, and RFC 0003 §5.4 makes `flush` the commit point for that reason. KIO is not that shape, and
the same section says so in the same breath: "a KIO `put` commits when the worker's `put`
completes (KIO's verb is already transactional)."

So this boundary states `pdfvfs_write` — one call, the whole file — and the staged four stay in
Rust where `pdf-fuse` reaches them. Four entry points nobody calls would be four more shapes to
keep frozen, and a C caller that used them would be inventing a transaction the protocol above it
does not have. `pdf_vfs::Vfs::write` already existed for exactly this caller; its doc comment names
KIO by name and predates this round.

### 5. `pdfvfs_split` is the third difference: the face has one question of its own

A mount is given one document. A URL is not: `pdf:/home/u/doc.pdf/pages/0007.pdf` is a file to open
and a path inside it, and **nothing but the file system can say where the boundary is** — `doc.pdf`
is a file and `pages` is not, and no rule about names could know that. `kio_archive` decides it the
same way (RFC 0003 §2), from the right, longest prefix first.

It is on this boundary rather than in the C++ for one reason and it is the round's own test
strategy: a machine with no KDE can still run it. `tests/` drives it in Rust and `c/` drives it
from C, so the one piece of logic the face adds is covered on every machine rather than on the
ones with KF6.

It answers a **length** rather than two strings — the caller already holds the path — so nothing is
allocated and no buffer crosses for the commonest call in the face.

### 6. The restriction level is an argument to `pdfvfs_mount_open`

`CLAUDE.md` principle 3: the policy is asked "once, in a place a host can supply". This is that
place, and all four levels are on the boundary from the first commit rather than the two that work
— which is the principle's actual requirement, that the shape not have to be revisited. A number
that is none of the four is refused with `EINVAL` rather than rounded to `off`: a host asking for a
level this build has never heard of has asked for something, and quietly giving it the permissive
one would be the program deciding to ignore a document's assertions on the host's behalf.

What `ask` reaches a caller as today, and why, is ADR 0869 §3.

## The self-checks, which are the point of a hand-written header

`include/pdf_vfs.h` is written by hand, as `viewer-ffi`'s is and for the same reason — it is the
artefact a plugin author reads, with the argument beside each shape, rather than a derivative of
Rust types that a person then has to read anyway. What a generator buys is that it cannot drift,
and that is bought back by two tests rather than assumed:

- `tests/header_and_library_agree.rs` reads the header and `src/abi.rs` as **text**: every
  `#[unsafe(no_mangle)]` name is declared exactly once in the header and no name is declared that
  the library does not export, and every `PDFVFS_` constant is the number the Rust gives it. The
  second is the one that would fail silently — a `#define PDFVFS_MEANS_DELETE_PAGE 3u` beside a
  Rust `2` is a plugin that compiles, links, runs, and tells a file manager that a page can be
  embedded. It also holds the two names this boundary hands a caller for the confined generator
  against `pdf_vfs`'s own constants, because a plugin looking for a program nobody builds would
  print a correct-looking sentence.
- `tests/a_c_program_drives_the_abi.rs` hands the header to a compiler and the symbols to a linker:
  `cc -std=c11 -Wall -Wextra -Werror` over `c/browse_a_document.c`, linked against the `cdylib` and
  run on a corpus document and a scratch copy of one. It catches what reading text cannot — a
  declaration that does not match by name, a struct tag colliding with a function in C's one
  namespace, a handle freed twice — and it drives every group of the boundary: the split, a
  listing, a `stat` whose size the read then matches exactly, the write meanings, three refusals
  with their sentences, the shortfalls, and both write verbs over the scratch copy.

Both skip loudly rather than failing where the tool is absent, which is `viewer-ffi`'s own rule for
a machine with no C compiler.

## Consequences

- **Nothing in the workspace changed.** `pdf-vfs` was not touched by this crate at all: every
  `errno`, every sentence, every path rule and every refusal is read out of it. That is the
  measurable form of RFC 0003 §7's "the faces contain *no* layout knowledge", and it is why the
  round could be spent on the boundary rather than on the tree.
- **A second consumer of the core exists and agrees with the first.** `pdf-fuse` and this crate
  call the same methods of `Vfs` and map the same `VfsError::errno`; where they differ is in what
  their protocol can carry, which is exactly the difference RFC 0003 §5.3 predicted.
- **`libpdf_vfs_ffi.so` joins `doc/todo/02` §5's list** for `libviewer_ffi.so`'s reason: it is not
  something a person runs, it is what a person links against, and a C or C++ program with
  `include/pdf_vfs.h` and no `-L` pointing into a build directory is the only way somebody outside
  this tree can try it.
- **The count is thirty-five and is asserted twice**, in the header check and in the `unsafe`
  position check, so an entry point added without a declaration fails one and an entry point added
  without an `unsafe` in its signature fails the other.
