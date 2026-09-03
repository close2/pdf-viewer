# 58 — The file-system faces: what RFC 0003 owes after its core

Status: **open**, the standing item of RFC 0003's stream, as `doc/todo/57` is of RFC 0002's.
Priority: 50-band — the core landed in session 899 and every remaining piece is blocked on a
decision, a toolchain, or the write side that the core's own table already describes.
Corpus witnesses: `doc/PDF20_AN001-BPC.pdf` (five pages, §12.3.3's outline, §12.4.2's labels);
`doc/PDF-Declarations.pdf` (two §7.11.4 embedded files whose names hold a COLON, so it is the
sanitisation witness); `doc/Tagged-PDF-Best-Practice-Guide.pdf` (images on pages 35, 36, 51, 60
and 63 of 72, and none on page 1); `doc/PDF20_AN002-AF.pdf` (a different document of a different
length, which is what the consistency tests need).
Clauses: §7.5.5 (the generation key's third component), §7.7.3.2, §7.7.3.4, §7.11.4, §8.9.5,
§12.3.3, §14.3.2, §14.3.3, §14.8.2.5.1.
Code: `crates/pdf-vfs/`, `crates/pdf-vfs/tests/a_face.rs`; ADRs 0840, 0841.

## What is done

Session 899, RFC 0003's first landing: **the shared core, read side only.**
`pdf_vfs::layout::LAYOUT` is §4's tree as one declarative table — path pattern, directory or file,
what generates it, and what writing and deleting it would each mean — and `path::resolve` is the
one place in the crate that reads a path's text. Six of the eight generators are a
`pdf_transform::Plan` and nothing else, and two assertions hold that to be true rather than said:
a page out of the mount is byte for byte `pdf_transform::apply`'s own piece, and a page's text is
`pdf_model::interpret`'s readback byte for byte. §5.4's generation key — `(mtime, size, §7.5.5's
last `startxref` offset)` — is asked before every answer and a change of it throws away the
worker, the inventories and every cached output; an open `Handle` keeps its bytes and its key, so
"no reader ever receives a splice of two generations" is a property of the type's shape.
§5.5's rule that a `stat` generates is implemented, with a byte-bounded cache keyed by
(generation, path) behind it. Every write is refused, and the two kinds are told apart:
§5.3's refusals-by-design each carry the sentence saying why they will still be refused when the
write side lands, and §5.2's five verbs are refused by the operation's own name. RFC §6's boundary
is `pdf_vfs::worker`'s two traits, with `InProcess` the unconfined implementation and the confined
one a transport change for the reasons ADR 0841 §2 states.

## 1. The departure the owner should overrule or ratify

**`images/` is a directory per page** — `images/0035/01.png` — where RFC §4 draws it flat as
`images/0035-01.png`. ADR 0841 §3 has the argument: a flat directory cannot be listed without
extracting every image in the document, because a file form depends on the codec *and* on whether
§8.9.6's mask travels beside it, and the alternative — predicting the names — makes a listing that
can name a file a read cannot produce. Per page, the listing and the read are one call and cannot
disagree. Nothing else in §4's layout moved.

## 2. The write side, which the table already describes

RFC §5.2's five verbs. Each row of `LAYOUT` already states what it means, so what is owed is the
transform call and the transactional shape, not a design:

- **Copy a PDF into `pages/`** — page insertion at the position the name states, through
  `pdf-transform`'s `pages --insert`, saved as §7.5.6's append. Note that the seam's `pages` verb
  reads *one* document and refuses a path in its range by name (ADR 0830), so an insertion from
  another file is `merge`'s shape and not `pages`'s; which of the two a `cp` into `pages/` becomes
  is the first thing this item has to settle.
- **Delete `pages/NNNN.pdf`** — page deletion, which `pages --delete` already is.
- **Copy into `attachments/`** — `attachments --attach`, which exists and writes an incremental
  update.
- **Delete an attachment** — `attachments --remove`, which exists.
- **Overwrite `meta/info.json`** — §14.3.3's entries, and RFC §9's open question 4 asks whether it
  belongs in v1 at all.

And the transactional half, which is the part with edges: RFC §5.4's commit point (a KIO `put`
commits when the worker's `put` completes; a FUSE write buffers and **commits on `flush`**, whose
error reaches the application's `close()`, where `release` reaches nobody), the renumbering that
follows any write to `pages/`, and the change notification that refreshes a file manager's stale
listing.

## 3. The two faces, in the RFC's own order

- **The FUSE face** (`pdffs <file.pdf> <mountpoint>`), on `fuser`'s pure-Rust `/dev/fuse` path —
  no C linkage. `doc/stack.md`'s rules decide the dependency, and RFC §7 notes that fuser's 0.17
  typed-API rework is recent and wants pinning and vendor-watching. What the face owes beyond the
  core: the inode table, the `notify_inval_entry`/`notify_inval_inode` invalidation **from a
  separate task** (issuing them synchronously from a request handler can deadlock against the
  kernel — a documented libfuse hazard), and logging each refusal's sentence, because FUSE returns
  `EPERM` with no message channel.
- **The KIO face** — a C++ `MODULE` plugin subclassing `KIO::WorkerBase` over a C ABI into this
  core, the `viewer-ffi` precedent. Toolchain risk moderate: CMake, Qt 6 and KF6 enter the build
  of that one component, which lives outside the cargo workspace.

## 4. The confined worker, which is the posture and not a nicety

`pdf_vfs::worker::InProcess` parses in the calling process. ADR 0841 §2 records what makes the
second implementation a transport change — no borrows in `Query` or `Answer`, one worker per
generation, the document handed over once as `FileBytes` the way ADR 0812's `SCM_RIGHTS` route
already does — and states the cost of not having it yet. **No face ships before it exists**: a
mount is entered by anything that touches a folder, and a file manager will open a document nobody
chose to open.

## 5. What the core still owes, each named in `Vfs::shortfalls`

- A §12.3.5 `/Collection` document is listed flat rather than under the folder schema its
  collection states, which RFC §4 asks for and which the viewer's sidebar already reads (ADR 0202).
- `text/document.txt` is built whole rather than streamed page by page, so its first byte costs
  the whole document. RFC §5.5 names the streaming.
- The cache has a memory bound and no disk half, which §5.5 offers as optional.
- An encrypted document opens only under §7.6.4.1's default user password. A worker is made per
  generation and `viewer_core::Secret` is deliberately not `Clone`, so a mount that survives a
  change of the file needs a design for re-supplying the password — a lending `Secret`, or a
  `SecretSource` a face implements. `doc/todo/57` §1 records the same shape for `merge`.
- A *listing* of `images/NNNN/` re-runs that page's extraction every time, because the listing is
  the extraction's own output names (which is what makes it exact); only a read puts the bytes in
  the cache, and a read puts the whole run's outputs there at once so that `cp -r` of one page's
  images costs one extraction. Caching the listing itself is a second kind of entry the cache does
  not have.
- **Nothing is measured.** There is no gate on this crate and no number anywhere: what a `stat`
  costs, what `ls images/` costs on a scanned book, what the cache's hit rate is under a `cp -r`.
  RFC 0002's suite got its perf floor in its second round (ADR 0801) and this one has none, which
  is the reason it is written here rather than assumed to be fine.
- A directory the document would fill past `Config::max_entries` is refused rather than truncated,
  and no document on this disk reaches it — so the ceiling is a decision without a witness.

## 6. RFC §9's open questions, none of them answered

The owner has not been asked again since approving the RFC. The seven stand: the scheme name
(`pdf:/` recommended), whether writes are opt-in per krarc's precedent, whether reorder-by-rename
is wanted at all, `meta/info.json` writes in v1, whether a content-destroying rewrite is something
RFC 0002 should offer so this face can refer to it, page-label alias symlinks, and the resolution
set for `renders/` — where 150 and 300 are what the core now states.
