# 0869 — The RFC's error mapping does not survive KIO, and the *ask* has a channel and no wire

Session 913. Status: **accepted**. The second of this round's two records: RFC 0003 §7's KIO face,
the three decisions KIO forces that the RFC did not anticipate, and the honest answer about
`CLAUDE.md` principle 3's *ask* level on the one face that has somewhere to put the question.
ADR 0868 is the C ABI underneath.

## Context

RFC 0003 §7 asks for "a deliberately dumb C++ `MODULE` plugin subclassing `KIO::WorkerBase`,
embedded JSON metadata (`"protocol": "pdf"`), built with CMake + extra-cmake-modules, linking
`Qt6::Core` + `KF6::KIOCore`, installed to `lib/qt6/plugins/kf6/kio/`", grades its toolchain risk
**moderate**, and puts it outside the cargo workspace so that "the workspace stays pure Rust and CI
treats the shim as an optional artefact built where KF6 exists".

That is what `kio/` is, and the plugin is dumb as asked: six hundred lines of C++ across a header and a source, no PDF, no layout
knowledge, and not one `errno` it chooses. Three things it *does* decide, because they are about
KIO rather than about documents.

## Decision

### 1. RFC §5.3's error mapping does not survive contact with KIO, and the sentence wins

The section says: "KIO reports refusals as `ERR_UNSUPPORTED_ACTION` / `ERR_WRITE_ACCESS_DENIED`
with the sentence."

**KIO does not work that way.** For almost every code in `KIO::Error`, the string a worker returns
is a *parameter* substituted into KIO's own canned message — `ERR_WRITE_ACCESS_DENIED` renders as
"Could not write to %1" — so returning our sentence in that slot produces "Could not write to
<two lines about why a page's text is not a byte stream>". The sentence is the whole point of
§5.3, and this mapping would destroy it while looking correct in the source.

`KIO::ERR_WORKER_DEFINED` exists for exactly this: its string is shown verbatim. So the mapping is

- **`ENOENT`, `EEXIST`, `EISDIR`, `ENOTDIR` keep a canned code** and are given the URL, because
  KIO's own words for those four are true, are already translated, and are shorter than ours;
- **everything else is `ERR_WORKER_DEFINED` with the core's sentence**, and the `errno` name is
  appended in brackets so that a script reading the error string still has the number the core
  chose.

This is the asymmetry that makes this face worth building, stated as code: a FUSE mount returns
`EPERM` and logs the reason where nobody reads it, and Dolphin shows the reason in a dialogue. The
harness prints all three of §5.3's refusals as the job's own `errorString`, and they are the core's
sentences, verbatim, to the em dash.

### 2. A listing does not `stat`, and a `mimetype` does not read

Two performance decisions that are correctness decisions once they are wrong.

**`listDir` fills in no `UDS_SIZE`.** RFC §5.5: "[d]irectory listings are cheap — names and types
come from the document's structure — and file managers stat lazily, so browsing stays fast and the
cost lands on the first touch of each file." A `listDir` that stated sizes would generate every
file in the directory, so `ls renders/300dpi/` on a 72-page book would rasterise 72 pages to
produce a column of numbers. `stat` does generate, because §5.5's other half is that an estimate
truncates: "the kernel clamps reads at the stated size … an under-estimate silently truncates a
page", and the same is true of a KIO `get`, which the harness checks by requiring the bytes it got
to be exactly the length the `stat` stated.

**`mimetype` is answered from the name.** KIO's documented fallback for a worker that does not
implement it is to issue a whole `get` — so hovering over a 300 dpi render would rasterise it. The
names in this tree all carry a true extension, which is what makes `QMimeDatabase::MatchExtension`
honest here rather than a guess.

**And the access bits are the core's answer**, not a list this file keeps: `pdfvfs_write_meaning` is
`pdf_vfs::layout`'s table speaking, so what a file manager greys out is the document's own shape and
a row added to the layout changes both faces at once.

### 3. The *ask* level: the channel exists, the wire does not, and the *warn* level is wired

This is the decision the round was asked to make, and the answer has two halves.

**What is wired.** `CLAUDE.md`'s *warn* level — the operation proceeds and the reasons are said
afterwards — reaches a person, today, through `KIO::WorkerBase::warning`, and the harness observes
it arriving at a KIO job as `KJob::warning`. It carries §7.5.6's own consequence on every deletion:
"[d]eleted objects shall be left unchanged in the PDF file, but shall be marked as deleted by means
of their cross-reference entries", so a person deleting a page is told that its content is still in
the file. That sentence is written where RFC §5.3 asks for it — "this RFC only insists the
refusal/behaviour be stated where the user deletes" — and this is the first face where it is
*shown* rather than logged.

**`warning()` rather than `messageBox()`, and that is not timidity.** A deletion always produces
that sentence, so a modal dialogue per `rm` is a face nobody keeps installed. KIO's non-modal
channel is what "the operation proceeded, and here is what the document said" wants; the modal one
is for a question, and a question is what *ask* is.

**What is not wired, and what it would cost.** `KIO::WorkerBase::messageBox` is a real question
channel and it is the reason this face was singled out. It is not enough on its own, because of
where the question is decided. RFC 0003 §6 puts *every byte of parsing* in a confined process, and
the restriction decision is inside it: `pdf_transform::apply` asks `pdf_model::restriction::decide`
once, at the seam, and under `Level::Ask` answers `Refusal::Unanswered` — "a pipe has nobody to put
the question to". The confined generator has no channel to a person **by construction**; that is
what confinement is.

So wiring it is not a shim change. It is one of two things, and both are the *core's* rather than
this face's, because RFC §7 forbids a face from knowing anything both faces do not:

- **make the wire a dialogue** — `crates/confined-transport` is one frame each way, and a worker
  that could interrupt an answer with a question would change the protocol both confined workers in
  this tree speak, including `pdf-view-worker`'s; or
- **ask first, in two round trips** — a `Query` that answers *would this operation be restricted,
  and with what reasons*, put to the worker before the operation; the broker asks the face, the
  face asks the person, and the operation is then issued at `Off` or refused. No protocol change,
  two extra crossings on a path a person is already waiting on, and one new thing that can be true
  between the two calls.

The second is the smaller and is the recommendation. Until one of them exists, `PDFVFS_RESTRICT_ASK`
is on the boundary and reaches a caller as `EACCES` with a sentence saying a question went
unanswered — which is `viewer_host::unanswerable`'s answer and `pdf-fuse`'s, unchanged, and is the
one degradation that is not "proceeding with the very thing the person asked to be consulted
about". `doc/todo/58` carries it as owed work with this ADR's two options named.

The plugin itself takes `PDFVFS_RESTRICT_OFF`, and that is a decision rather than a default nobody
made: principle 3 says a document's restrictions are the reader's and that turning them off "shall
always be possible", there is no interface in this face to choose otherwise yet, and the level that
is the reader's own is therefore the one it takes.

## How the build stays optional, and what was actually driven

**Optional, checked rather than claimed.** `kio/` has no `Cargo.toml`, is named by no manifest and
reached by no build script; `members = ["crates/*", "tools/*"]` cannot see it. `cargo build`,
`cargo clippy --workspace --all-targets` and `cargo nextest run --workspace` on a machine with no
KDE are untouched by its existence. What builds it is
`crates/pdf-vfs-ffi/tests/the_kio_worker.rs`, which **skips, loudly, printing what CMake could not
find**, where `cmake`, ECM, Qt 6 or KF6 is absent — and *fails* on any other configure error,
because a plugin that will not compile against a toolchain that is there is a defect rather than an
absence. That predicate was run against the defect before it was believed (trap 13): a copy of
`kio/` whose `find_package` names a package that does not exist produces "Could not find a package
configuration file", which is what the skip matches.

**What was driven, and what was not.** `kio/test/drive_the_worker.cpp` is a KIO **client** — a
`QCoreApplication` running the same jobs Dolphin runs — so the plugin is loaded by KIO, from its
embedded metadata, into a forked `kioworker`, and every command crosses a socket. It lists the
root and `pages/`, `stat`s a page and `get`s exactly the bytes the `stat` promised, takes all three
refusals, deletes a page and inserts one through KIO's own `del` and `put`, and watches the
listing renumber from five to four to five. That is the real plugin through the real protocol on a
machine with no KDE session.

**It is not Dolphin, and this is the honest limit.** Nothing here says anything about how a listing
is rendered, about the `archiveMimetype` association that makes a click on a PDF enter it as a
folder rather than open it, about drag and drop, or about a person's experience of any of it. Those
need a session, and the agent's account on this machine has none it may use for this. The two
things a session would test that nothing here does are the *entry* — the mime association — and the
*look*, and both are named in `doc/todo/58`.

One thing the harness found that is worth writing down because it would otherwise be read as a
defect in the worker: **KIO appends a "." entry to every listing itself.** `pdfvfs_list` never
answers one and the worker sends exactly what the core says, so the harness drops it; a count that
did not would be one larger than the document's on every directory.

## Consequences

- **RFC 0003 §7's second face exists**, and the RFC's own construction order is vindicated: the
  shim is thin because the core was built first, and the layout it serves is the one `pdf-fuse`
  already serves.
- **The RFC's §5.3 sentence about `ERR_UNSUPPORTED_ACTION` is wrong and is corrected here rather
  than in the RFC**, which is accepted and stands as written; `doc/todo/58` carries the correction
  where a reader of the item will meet it.
- **A toolchain assumption enters the tree**: CMake, extra-cmake-modules, Qt 6 and KF6 KIOCore, for
  one component nothing else depends on. `doc/stack.md` records it with the rule that keeps it
  optional.
- **The *ask* level is now a named piece of owed work with two costed options** rather than a
  sentence in a header saying it cannot be done.
