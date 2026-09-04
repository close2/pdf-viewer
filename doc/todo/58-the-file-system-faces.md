# 58 — The file-system faces: what RFC 0003 owes after its core

Status: **open**, the standing item of RFC 0003's stream, as `doc/todo/57` is of RFC 0002's.
Priority: 50-band — the core landed in session 899, the confined worker in 902, the write side in
906, the FUSE face in 909 and **its first mount in 911**, so what is left is the **second** face, a
decision the owner has not been asked, and the read side's corpus walk.
Corpus witnesses: `doc/PDF20_AN001-BPC.pdf` (five pages, §12.3.3's outline, §12.4.2's labels);
`doc/PDF-Declarations.pdf` (two §7.11.4 embedded files whose names hold a COLON, so it is the
sanitisation witness); `doc/Tagged-PDF-Best-Practice-Guide.pdf` (images on pages 35, 36, 51, 60
and 63 of 72, and none on page 1); `doc/PDF20_AN002-AF.pdf` (a different document of a different
length, which is what the consistency tests need).
Clauses: §7.5.5 (the generation key's third component, and Table 15's `/Info`), §7.5.6, §7.6.3.1,
§7.7.2, §7.7.3.2,
§7.7.3.3, §7.7.3.4, §7.9.4, §7.11.4, §8.9.5, §12.3.3, §12.4.2, §12.5.6.15, §12.7.4.2, §14.3.2,
§14.3.3, §14.3.4, §14.7.5.4, §14.8.2.5.1.
Code: `crates/pdf-vfs/`, its `tests/a_face.rs`, `tests/a_write.rs`, `tests/write_corpus.rs`,
`tests/confined.rs` and `examples/vfs_cost.rs`, `crates/pdf-fuse/` and its `tests/a_face.rs`,
`crates/confined-transport/`, `crates/pdf-transform/src/update.rs`;
ADRs 0840, 0841, 0846, 0847, 0854, 0855, 0860, 0861, 0864, 0865.

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

Session 902, RFC 0003's second landing: **the confined worker**, which §4 below used to be about.
`pdf-vfs-worker` is a separate program that confines itself before it reads a byte, takes the
document as ADR 0812's descriptor, and answers `Query`s over the wire `crates/confined-transport`
now holds for it and for `pdf-view-worker` both (ADR 0846) — `viewer-confined` was moved onto that
crate in the same round, because a shared thing nobody has moved onto is a third copy. The
allow-list is `Profile::Interpreter` unchanged, measured with `strace -ff` rather than assumed; ADR
0847 §1 lists the twenty-three calls the worker makes behind the filter and why each was already
there. The allow-list found a defect on the way: `pdf_transform::render` asked the machine how many
cores it had, which is an `openat` a confined process is killed for, so `RenderPlan` gained
`strips` (ADR 0847 §2). Six probes hold the boundary rather than describing it, and a worker that
dies is a named error with a fresh worker behind it rather than a hang.

Session 906, RFC 0003's third landing: **the write side, and the transaction around it.** All five
of §5.2's verbs work — a PDF copied into `pages/` inserts its pages at the position the name
states, `rm pages/NNNN.pdf` deletes one, a file copied into `attachments/` is embedded, deleting
one removes it, and `meta/info.json` is overwritable, which answers §9's fourth open question
*yes*. What the round had to build first is the reason it is a round rather than plumbing:
`pages` and `merge` write a *new* file and `CLAUDE.md` permits only an append to a document
somebody has open, so `pdf_transform::update` is a fourth writing verb — §7.7.3.2's tree edited in
place by §7.5.6's update, and §14.3.3's entries set the same way. The commit is atomic
(temporary, sync, `rename(2)`), the broker checks §7.5.6's own prefix property against the file
before writing a byte, the generation transition after our own commit is `Provenance::Ours` rather
than looking like somebody else's edit, and a write staged against a generation that has gone is
`ESTALE` rather than a clobber. `create`/`write_at`/`flush`/`release` are the POSIX transaction,
a staged write is visible in the tree and absent from the document, and every refusal has an
`errno` the core names. ADRs 0854 and 0855.

Session 909, RFC 0003's fourth landing: **the write side's corpus walk, and the FUSE face.**
`crates/pdf-vfs/tests/write_corpus.rs` drives all five verbs over every corpus document the core
opens, on a fresh backing per verb, and holds §7.5.6's prefix property read off the file, the
document re-opening at the page count the edit stated, the renumbered listing, §14.7.5.4's carried
key, and every surviving page's raster against the page it was — a comparison *between different
ordinals*, which is what makes it a check of "an ordinal is a position". It found three defects
(ADR 0860): a node with no `/Count` counted as zero, so an insertion under it left a two-page
document reading as one; two documents whose catalog does not reach the tree being spliced, where
an insertion "before page 1" came back after it and a second one changed nothing at all; and
§7.5.6's "a deletion does not destroy bytes" said on the page verb and not the attachment one. All
three are fixed. Sixteen documents write two different files for one plan and that is §7.6.3.1's
requirement rather than a defect — a fresh random initialisation vector in front of every AES
string — so what binds there is the length. And `crates/pdf-fuse` is the first *face*: `pdffs
<file.pdf> <mountpoint>` on `fuser`'s pure-Rust path, pinned `=0.18.0`, four new locked packages,
no C linkage, no layout knowledge, an inode per name, every refusal logged as a sentence as well
as returned as a number, and RFC section 5.4's invalidation on a thread of its own (ADR 0861).

Session 911, RFC 0003's fifth landing: **the face was mounted, by hand, and ten things were
wrong.** ADR 0864 has the seven the kernel found and ADR 0865 the three that were underneath the
face and had nothing to do with FUSE. The largest is that `cp new.pdf pages/0004.pdf` — the verb
RFC §5.2 leads with — could not be done at all, because `cp` issues `open(O_WRONLY|O_TRUNC)` on a
name that exists and the face answered every access mode with a read handle. Beside it: the
kernel's own `O_TRUNC`, which is a `setattr` with no file handle, accepted and ignored; a `close`
of a file nothing was written to committed zero bytes and failed "not a PDF"; `touch` succeeding
and changing nothing while the comment above the guard said it could not; every file owned by
`root` and dated 1 January 1970; an `ENOENT` logged as though it were a refusal, so a file
manager's `.directory` probes flood the only channel there is; and two writes in flight in one
mount losing the second, quietly. Underneath: a page with **two** images killed the confined
worker with `SIGSYS` on `openat`, which is `glibc` sizing a per-thread allocator arena from
`/sys/devices/system/cpu/online` on `rayon`'s pool thread; every page of ISO 32000-2 refused to
come out of `pages/`, under a false sentence hiding a real ordering defect in §14.7's carry; and a
second `ls -l` of a thousand-page document cost more than the first. All ten are fixed and each
has a test.

## 1. The departure the owner should overrule or ratify

**`images/` is a directory per page** — `images/0035/01.png` — where RFC §4 draws it flat as
`images/0035-01.png`. ADR 0841 §3 has the argument: a flat directory cannot be listed without
extracting every image in the document, because a file form depends on the codec *and* on whether
§8.9.6's mask travels beside it, and the alternative — predicting the names — makes a listing that
can name a file a read cannot produce. Per page, the listing and the read are one call and cannot
disagree. Nothing else in §4's layout moved.

## 2. The write side — **done in session 906**, and what is left of it

RFC §5.2's five verbs all work, and §5.3's four refusals each carry their own `errno`. What this
item still owes on the write side is three things, none of them blocking a face:

- **An attachment cannot be replaced in place.** A write to a name §7.7.4's tree already files is
  `EEXIST`; replacing would be a removal and an embedding, which is two updates, and this verb
  writes one. A face that wants `cp -f` to work needs the pair designed as one transaction.
- **An in-place insertion carries the pages and not what the incoming document says about them.**
  Its `/AcroForm`, `/OCProperties`, `/Outlines`, `/Names` and `/StructTreeRoot` are each named in a
  warning rather than reconciled, and `/StructParents` is stripped from every carried page.
  `merge` reconciles all of those and cannot be used here because it rewrites; whether the two
  engines can be made one is `doc/todo/57`'s question as much as this one's.
- **`pdf-transform` has a fourth writer and no fourth CLI verb.** RFC 0002's verb set is the
  owner's, and `update` is reachable from the library alone. A command line for it would give the
  transform suite a way to edit a file in place, which is a different offer from every verb it has.

## 3. The two faces, in the RFC's own order

- **The FUSE face — done in session 909** (ADR 0861), **and mounted by hand in 911** (ADRs 0864,
  0865). The commands, and what each of them came back with, are in `doc/history/911-*.md`; the
  paragraph below is kept because its argument still binds — a mount stays out of every gate, and
  the *next* mount by hand is owed the same way this one was, on the next round that changes the
  face.
  **The first mount found ten defects, and it is the argument for the exercise rather than a
  complaint about the tests.** `crates/pdf-fuse/tests/a_face.rs` drives the face the way a
  kernel drives it and holds everything between the kernel's verb and the core's answer, and a
  *mount* is deliberately not in any gate — `fuser`'s pure-Rust path asks the kernel for
  `/dev/fuse` and runs `fusermount3`, so a gate that mounted would be measuring the machine's
  kernel configuration, its `fuse` group and `/etc/fuse.conf` rather than this tree (the same
  argument `viewer-ffi`'s C-compiler gate makes for skipping). **What is owed is a mount by hand,
  written up**: `pdffs doc.pdf mnt/`, then `ls`, `cp`, `grep -r`, `cp` a PDF into `pages/`, `rm`
  one, and `fusermount3 -u`. Until somebody has done that, the wire format between
  `fuser`'s reply objects and a real kernel is the one part of this face nothing has exercised.
  **The three smaller questions are answered.** `readdir`'s offsets survive a directory larger
  than one reply buffer: 1023 names came back once each, in order, no duplicate and no gap. A zero
  attribute timeout is *not* what a stat storm costs — §5.5's rule that a `stat` generates is, and
  §5's last bullet has the number. And `--allow-other` cannot be used on this machine, which it
  says: `fusermount3: option allow_other only allowed if 'user_allow_other' is set in
  /etc/fuse.conf`, which is commented out here; nothing in this tree can change that and nothing
  should try.

  **What the face still owes**, all found by the mount and none of them fixed:
  - **`mv` out of the mount deletes the page**, because `mv` across file systems is a copy and an
    `unlink`, and `unlink` in `pages/` is §5.2's deletion verb. That is the correct composition of
    two verbs this face has and it is the archive-manager convention — but a person dragging a
    page out of a file-manager window will not have read either, and §7.5.6's sentence goes to a
    log they are not looking at. Whether a face should refuse a cross-device rename is a decision
    nobody has made.
  - **An attachment cannot be replaced, and the `errno` a person sees for trying is `EPERM`
    rather than the `EEXIST` §2 above documents** — because `cp` onto an existing name goes
    through `open` and not `create`, and `open` stages before the name is compared. The verb is
    still the missing one; only the number moved.
  - **`cp -r` out of the mount makes a copy nobody can delete.** The derived files are `0444` and
    the derived directories `0555`, which is the layout table answering honestly about *the mount*
    — and `cp -r` preserves them, so `rm -r` on the copy needs a `chmod -R u+w` first. Whether a
    face should report a mode a copy can live with, or whether that would be lying about what the
    mount permits, is a decision nobody has made.
  - **`: > mnt/pages/0002.pdf` is loud in the log and silent in the shell**, because `bash` does
    not check `close(2)`. Nothing can be done about the shell; what could be done is refusing the
    truncation itself, which would make a legitimate `cp` fail. Stated rather than chosen.
- **The KIO face** — a C++ `MODULE` plugin subclassing `KIO::WorkerBase` over a C ABI into this
  core, the `viewer-ffi` precedent. Toolchain risk moderate: CMake, Qt 6 and KF6 enter the build
  of that one component, which lives outside the cargo workspace.

## 4. The confined worker — **done in session 902**, and what is left of it

`pdf_vfs::ConfinedWorkers` is RFC §6's worker: a separate program under seccomp-BPF, Landlock and a
4 GiB ceiling, holding the document as a descriptor it could not have opened, answering the same
`Query`s with the same `Answer`s as `InProcess` — which `tests/confined.rs` asserts question by
question, both ways, over a real document. A face chooses it in one line and inherits the posture.
ADR 0847 is the record: the allow-list, what crosses and its bound, what a death becomes.

**And one thing it does not have, which is the fourth of the three below and the largest**: a
confined worker has **no fonts**. ADR 0870 is the record — a document naming a face the machine has
and the document does not embed sent `pdf_font::substitute` walking `/usr/share/fonts`, and
`SECCOMP_RET_KILL_PROCESS` ended the worker rather than returning the `Err` that code is written to
shrug off. It is stated now (`no_machine_fonts`, before the confinement, in both workers), so the
worker lives and draws from the compiled-in faces; **what is owed is the fidelity**, and the shape
of the answer is the one ADR 0812 already used for the document: the broker is unconfined, so the
broker hands the face across. Until it does, a confined mount and the confined viewer draw a
CJK or Arabic page the way a machine with no such font installed draws it.

Three things this item still owes, none of them blocking a face:

- **A worker is per generation and per `Vfs`, so a face mounting a hundred documents starts a
  hundred processes.** That is the right shape for a mount of one and an open question for a file
  manager browsing a directory of PDFs; nothing here pools, reaps on idle, or bounds how many may
  live at once. `RLIMIT_NOFILE` is 8 in the confinement, so the *worker* side is already bounded;
  the broker side is not.
- **A killed worker is retried on the next operation and not on the current one.** `Vfs::current`
  asks `Worker::is_alive` beside the generation key, so the operation after a death gets a fresh
  worker — but the operation that *found* the death still fails, and a face has to decide whether
  to show that or to retry once. Stated rather than chosen, because it is a face's policy.
- **A `Canceller` reaches the worker and nothing reaches it through `Vfs`.** `Confined::canceller`
  exists and `Vfs` holds `Box<dyn Worker>`, so a face that wants to end a render a person navigated
  away from has to hold its own factory the way `tests/confined.rs` does. That is a small piece of
  `Vfs` API and it wants a face's requirement to shape it.

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
- **Both sides now have a corpus walk** — the write side in session 909 (ADR 0860), the read side
  in session 914 (ADR 0871), each with its line in `doc/todo/02` §2. `tests/read_corpus.rs` lists
  the whole layout for every corpus document, `stat`s every entry, reads every file and holds each
  against the generator the table delegates to, over the **confined** transport. What it found on
  its first sixty documents is ADR 0870 and is below. Two things it does *not* measure, and they
  are the shortfalls this round created rather than closed:
  - **The substitution gap ADR 0870 opened.** A confined worker has no machine fonts, so a document
    naming an uninstalled face is drawn from the compiled-in ones; the walk puts its own process in
    that posture so that its columns measure the transport rather than two machines. **What closes
    it is the broker supplying the face** — the broker is unconfined, it already hands the document
    across as a descriptor (ADR 0812), and a face is a file like any other. Until then a confined
    mount and a confined viewer draw such a page the way a machine with no fonts installed draws
    it, and `pdf_model::interpret` says so per font (§9.10.2's coverage note).
  - **The tail of long documents.** `PAGES_READ` bounds the pages whose files are read; the
    listings are always whole. The corpus's own distribution is why the figure is affordable, and a
    population with more long documents — `doc/todo/03`'s crawls — would be a stronger denominator
    for `renders/` exactly as it would for the deletion verb below.
- **`Plan::Update`'s output is read by nobody else.** `pdf-transform`'s `foreign_corpus` puts the
  other five writers' output through `qpdf --check`, `pdftoppm` and `mutool draw`; the in-place
  update is not in that walk, so the only reader that has ever judged it is this one. RFC 0002
  section 9's fourth layer is exactly the instrument for it and the walk exists to be extended.
- **The deletion verb is measured on a tenth of the corpus.** 883 of the 974 documents have one
  page, and `update` refuses to delete a document's last one; the walk counts the refusal by name
  and the number is in its own output. A population with more multi-page documents — the SafeDocs
  crawl, `format-corpus` — would be a stronger denominator for that one verb.
- **A `stat` generates, and on a large document that is minutes.** `ls -l pages/` on ISO
  32000-2's 1023 pages took **2 min 45 s** (round 911), which is 1023 page extractions at about
  160 ms each, and the pieces are about **1.8 MB** apiece — the closure of a heavily shared
  document — so `pages/` reports 1.9 GB for a 12 MB file. A second listing is now free (ADR 0865
  §3 put the sizes in the cache past eviction), and the *first* one is the shortfall: a file
  manager opening such a mount waits three minutes. What would answer it is a generator that can
  state a piece's length without writing it, which nothing in the transform layer offers, or a
  listing that reports no size until asked — which §5.5 forbids for the reason ffmpegfs paid for.
- **There is still no gate, and there are now numbers.** `crates/pdf-vfs/examples/vfs_cost.rs`
  prints, per document: a worker per generation in each transport, one question of each shape in
  each transport, the largest answer and the bound past which the confinement refuses one, and a
  `stat` that generates beside the same `stat` cached. Session 902's run is in that round's record.
  What is *not* measured, and what the next round of this stream should take: `ls images/` on a
  scanned book, the cache's hit rate under a `cp -r` of one directory, and `text/document.txt` on a
  long document — which is the streaming shortfall above, priced. And none of it is a floor: RFC
  0002's suite got its perf floor in its second round (ADR 0801) and this crate still has no line in
  `doc/todo/02` §2 beyond the core's.
- A directory the document would fill past `Config::max_entries` is refused rather than truncated,
  and no document on this disk reaches it — so the ceiling is a decision without a witness.

## 6. RFC §9's open questions — one answered, six standing

**The fourth is answered: `meta/info.json` writes are in v1, yes.** Session 906 implemented them
and ADR 0855 §5 has the argument — the file *is* §14.3.3's Table 349, the write is the read's
inverse, and reading the file and writing it straight back changes nothing the document states.
The owner may of course overrule it; what it is not any more is undecided.

The other six stand, and the owner has not been asked since approving the RFC: the scheme name
(`pdf:/` recommended), whether writes are opt-in per krarc's precedent — **now a live question
rather than a hypothetical, because the writes exist** — whether reorder-by-rename is wanted at
all, whether a content-destroying rewrite is something RFC 0002 should offer so this face can
refer to it (also now live: a page deleted through the mount keeps its bytes, and the deletion
says so), page-label alias symlinks, and the resolution set for `renders/`, where 150 and 300 are
what the core states.

**And a seventh the write side raised**: §5.3's refusals return `EACCES` for both `on` and *ask*,
because a file system has nobody to ask. A KIO worker *can* put a question to a person — Dolphin
will show a dialogue — so a face with a channel could implement the *ask* level properly and
re-issue the write. Nothing in the core has to change for it; whether it is wanted is the owner's.
