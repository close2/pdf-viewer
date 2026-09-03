# 0841 — The broker holds no document, and a flat directory cannot be listed

Session 899. Status: **accepted**. The second decision record of RFC 0003's implementation: where
the confinement boundary is drawn before there is a face to confine, and the one place this
landing departs from the approved layout.

## Context

RFC 0003 §6 is the part of that document with teeth, and it says why: "[t]he frontend processes
are, by construction, fed hostile bytes by anything that opens a folder. KIO runs workers out of
process (the `kioworker` host executable) but **applies no sandbox to them** — no seccomp, no
Landlock; a worker has the user's full privileges. A FUSE daemon likewise. So the posture is the
one this tree already built for its windows (ADR 0713, `viewer-confined`)."

A mount is the most exposed surface this project would ship. It is long-lived, it is entered by
anything that touches a folder, and a file manager will open a document nobody chose to open. The
question this round had to answer is not *whether* to confine — it is where to put the boundary
**now**, in a round that ships no process split, so that adding the split later is a transport
change and not a rewrite.

## Decision

### 1. The boundary is a trait, and the broker is on the wrong side of it by construction

`pdf_vfs::worker` states RFC §6's split as two traits and two data types:

- `Query` — one question about the document, plain data, no borrow, no path, no descriptor.
- `Answer` — one answer, the same.
- `Worker` — answers questions about **one generation of one document**.
- `Workers` — makes a worker for a generation. Separate from `Worker` because the lifetime is the
  design: one worker per generation, so a document that changed under the mount is a *new* worker
  rather than a worker asked to change its mind.

`Vfs` — the broker — holds no `pdf_syntax::Document`, calls no reader, and decodes nothing. What
it holds is paths, the layout table, the cache and the wire types. That is RFC §6's sentence made
structural rather than promised: "[t]he frontends and the core never parse PDF bytes."

The one thing the broker does look at is the file's **tail**, for §7.5.5's `startxref` offset,
and it is a fixed 4096-byte window scanned for an ASCII keyword. That is not a parse, and it is
the only byte-level thing on this side. Everything else about the file — its page count included —
arrives as an `Answer`.

### 2. `InProcess` is the unconfined implementation, and what makes the confined one a transport
   change is stated rather than hoped

`InProcessWorkers` answers in the calling process. That is what a test harness and a first face
need, and it is the same default `pdf-transform` itself took (RFC 0002 §13 question 3, ADR 0800
§6). Three properties are what make the confined implementation a second `impl` rather than a
redesign, and each is a fact about the code as written:

- **Nothing in `Query` or `Answer` borrows anything.** A `Query` is a page number, a resolution or
  a name; an `Answer` is a count, bytes, or a map of names to bytes. Both cross a pipe unchanged.
- **A worker is handed its document once, at spawn, as `pdf_syntax::FileBytes`.** That is exactly
  the shape ADR 0812 built for `pdf-view-worker`: the broker opens the file, passes the descriptor
  with `SCM_RIGHTS`, and the worker receives it through `FileBytes::from_handle` — which takes the
  length the host measured, because the confined side's filter admits `pread64` on that descriptor
  **and not `fstat`**, a call that takes a path and would be a question about the file system.
  Nothing is widened: the two syscalls ADR 0812 admitted are the two a `pdf-vfs` worker needs.
- **A worker's life is one generation.** So the moment a confined implementation would fork and
  hand over a descriptor already exists in the control flow, at `Workers::spawn`, and is reached
  from exactly one place.

What the broker would pass, in the write direction, is stated by RFC §6 and is not implemented
this round: "the confined worker *computes* the transform output (the §7.5.6 append bytes); the
broker validates the frame (length, magic) and performs the actual file append — the worker keeps
no filesystem, the broker keeps no parser." The read side needs none of it, and writing it before
a write verb exists would be a wire nobody had used.

### 3. `images/` is a directory per page, and that is a departure from RFC §4

RFC §4 draws `images/` flat: `0001-01.jpg`, `0003-02.png`, page ordinal and index within the page.
This landing draws it `images/NNNN/MM.ext`.

The argument is the RFC's own principle, one clause later. §5.1: "nothing walks the whole document
to answer `ls doc.pdf/`". A flat `images/` cannot be listed without knowing every image's file
form, and a file form is decided by the image's codec **and** by whether §8.9.6's mask travels
beside it — which is known only once the image has been extracted, because a `/Mask` may be
§8.9.6.3's explicit mask (an image, so a sidecar file) or §8.9.6.4's colour key (a range of sample
values, so no file). So the flat listing has two possible shapes and neither is acceptable:

- **Extract everything to list it.** `ls images/` decodes every image in the document. On a
  seventy-two-page guide that is tolerable; on a scanned book it is a mount that hangs.
- **Predict the names.** Then a listing sometimes names a file a read cannot produce, and `cp -r`
  fails on a name the directory itself gave.

Per page, both problems disappear at once, and they disappear for the *same* reason: the listing
of `images/0035/` and a read out of it are **one call** — the names are the names
`pdf_transform::images` gave its outputs, so the two cannot disagree by construction. `ls images/`
is then the page count and nothing else, like every other directory in this tree. The naming
information RFC §4 asked for is unchanged; only the solidus moved.

It is a departure from a document the owner approved, so it is recorded as one: `doc/todo/58`
carries it as the first thing for the owner to overrule, and the layout module states it where a
reader of the table will meet it.

## Consequences

- The FUSE face can be written without deciding anything about confinement, and the confined
  worker can be written without touching the face: both are `pdf_vfs::worker`'s neighbours.
- **The in-process worker is not confined, and that is a stated cost rather than an omission.**
  Until the second `impl` exists, a `pdf-vfs` consumer parses hostile bytes in its own process
  with its own privileges — which is what every consumer of `pdf-transform` already does, and
  which is exactly why RFC §6 exists. The cost is bounded today by `pdf_syntax::Limits`, by the
  render pixel ceiling, and by the image codecs already going through the confined `pdf-sandbox`
  worker; it is not bounded by a kernel filter. No face ships until it is.
- `pdf-transform`'s codec sandbox is inherited rather than bypassed: `images/` goes through
  `pdf_model::image::decode`, so JBIG2, JPX and CCITT are decoded in `pdf-sandbox`'s worker
  exactly as in the viewer, and a build without that worker beside it refuses those images by
  name.
