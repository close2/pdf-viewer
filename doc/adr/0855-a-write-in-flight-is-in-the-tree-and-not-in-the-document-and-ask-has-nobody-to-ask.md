# 0855 — A write in flight is in the tree and not in the document, and *ask* has nobody to ask

Session 906. Status: **accepted**. The second of this round's two records: what a POSIX write
looks like from inside a document-as-a-directory, what each refusal becomes on the way out, and
RFC 0003 §9's fourth open question, answered.

## Context

A command line's write is one call. A POSIX write is four or more — `create`, `write` several
times, `flush`, `close` — and the bytes that decide whether the operation is possible at all
arrive in the middle of it. Everything a file-system face has that a `pdf-transform` invocation
does not is in that gap: a partial file that is somewhere, a listing that has to show or not show
it, a writer that can be killed, a second reader looking while it happens, and an error that has
to come out as a number.

RFC 0003 §5.4 chooses the moment — a FUSE write buffers, validation and commit happen on `flush`,
whose error return reaches the application's `close()`, while `release` reaches nobody and is
therefore only cleanup. What it does not choose is the four things below.

## Decision

### 1. A staged write is in the tree, and it is not in the document

`Vfs::create` starts an in-memory staging buffer and hands back a token; `write_at` fills it;
`flush` makes it real; `release` drops it. While it is staged the name **appears in its
directory**, `stat` answers the length written so far, and a read of it gives those bytes back.

That is not a convenience. Every copying tool stats its destination after writing, and a tree that
named nothing there would fail the copy it had just accepted. It is also what a real file system
does with a half-copied file, which is the whole design principle of this face: the answer a
person already knows should be the answer they get. A second reader sees the same thing — the
document as it was, plus a file on its way in.

The document itself is untouched until the flush. Nothing is written, nothing is reserved, and a
mount of a read-only file will not discover that until the flush either — which is correct, because
that is when a real write would discover it.

### 2. What happens when `flush` never comes, and the honest limit

Nothing. The bytes are dropped, the name leaves the listing, and the file on disk is byte for byte
what it was; `Abandoned::sentence` is what a face logs so the disappearance is explained rather
than discovered.

**But an abandoned write is not what a killed writer produces**, and that distinction is the one
worth writing down. The kernel calls `flush` on a killed process's descriptors too. So what stops
a torn copy from becoming a torn document is *validation*, not the absence of a flush:

- A truncated PDF copied into `pages/` **is opened by this tree's recovery scanner**, which is the
  round's one surprise. Two thirds of a fourteen-page document opens with the eight pages the scan
  found, and the insertion succeeds. The round expected a refusal and the tree was right: a
  truncated copy and a damaged document somebody meant to insert are the same bytes, and nothing
  anywhere knows how long the copy was meant to be. Refusing would mean declining a real file. So
  the insertion proceeds and the recovery is named in a warning (trap 5).
- A file that is not a document at all is refused, and nothing is written.
- A truncated file copied into `attachments/` is embedded truncated — exactly as `cp` to a real
  file system leaves a truncated file.

### 3. Every refusal is an `errno`, and the numbers live in the core

RFC 0003 §7 keeps every decision about the tree out of the faces, and this is one of them: a KIO
worker maps these onto `KIO::Error` and a FUSE daemon hands them to the kernel, and two faces
disagreeing about what a refused write *is* would be the second copy of a decision. `VfsError::errno`
is the one place.

| refusal | `errno` | why |
|---|---|---|
| a path the layout does not name, or this document does not have | `ENOENT` | including a `pages/` position past one past the end |
| a derived file — `renders/`, `meta/xmp.xml`, `meta/outline.json` | `EROFS` | RFC 0003 §5.3's own word: a read-only view of something else |
| `text/`, `images/`, a directory itself, a rename inside `pages/` | `EPERM` | §5.3's semantic refusals: this program will not do it |
| the document restricts the operation and the level is `on` or `ask` | `EACCES` | §4 below |
| a name §7.7.4's tree already files | `EEXIST` | a replacement is two updates and this verb writes one |
| a name a directory cannot hold | `EINVAL` | the listing and the read have to agree |
| the document changed under a staged write | `ESTALE` | ADR 0854 §4 |
| more bytes than one write in flight may hold | `EFBIG` | an explicit budget, principle 3 |
| the document, or the file being written into it, could not be read as one | `EIO` | the request was well formed and the bytes were not |
| the document would fill a directory past the ceiling | `EOVERFLOW` | a truncated listing is a wrong answer that looks right |

FUSE carries no message beside the number, so a face logs `VfsError::to_string()` with every one
of them — §5.3's own instruction, and the reason the sentences exist in the core rather than in
either face.

### 4. *Ask* is answered as a refusal, and that is a decision

`CLAUDE.md` principle 3's four levels are `off`, `on`, *ask before the operation*, and *warn
before the operation*. The policy is asked exactly once and in the place it already was — inside
`pdf_transform::apply`, per document — and RFC 0003's core supplies it through `Config::policy`.
Nothing in `pdf-vfs` reads a Table 22 bit.

Three of the four levels need no decision here. `off` proceeds and is the default, because the
program is the reader's and `CLAUDE.md` makes that level the one that "shall always be possible".
`on` refuses, which is §7.6.4.1's own `shall` kept. `warn` proceeds and the reasons come back in
`Committed::warnings`.

**A file system has no dialogue, so *ask* has to be answered here.** It is answered as a refusal,
never as a silent proceed — which is the choice `viewer_host::unanswerable` already makes for a
host that cannot put the question, and the only one that does not do the very thing the person
asked to be consulted about. It is a separate `WorkerError` variant from `on`'s, so the two
sentences exist to be logged, and both become `EACCES`: there is no number for "somebody would
have been asked", and that is FUSE's poverty rather than a decision of ours.

What this leaves open is exactly what the four levels were designed to leave open. A face with a
channel back to a person — a KIO worker can raise a dialogue, and Dolphin will show it — can put
the question and re-issue the write. Nothing in the core has to change for that: the refusal
carries the reasons, which is what `pdf_model::restriction` answers with and why it answers with
reasons rather than a verdict.

### 5. RFC 0003 §9's fourth open question: **yes**

`meta/info.json` is writable in v1. The argument is that the file *is* §14.3.3's Table 349 and
the write is the read's inverse, which makes three things true at once:

- **It is idempotent with the read.** Reading the file and writing it straight back changes
  nothing the document states. That is asserted, and it is what makes the pair one thing rather
  than two.
- **The mapping is total and needs no invention.** Table 349's nine keys, `/Trapped` written as a
  name because the row says so twice ("This shall be the name True , not the boolean value
  true ."), everything else a §7.9.2.2 text string because §14.3.3 makes that a `shall`, and the
  two dates held to §7.9.4's unconditional half — "[t]he prefix ' D: ' shall be present, the year
  field (YYYY) shall be present". A key the table does not define is refused rather than written.
- **Omission means removal, and non-Table-349 keys are untouched.** The file is the whole of the
  table and nothing else, so a key it omits is an entry the document no longer states; a key the
  *dictionary* holds that the table does not define is left exactly as the producer wrote it,
  because this file does not show it and is therefore not where it would be deleted from.

Two clauses had something to say and were read rather than assumed. §14.3.3 deprecates the
dictionary in PDF 2.0 "except for two entries, CreationDate and ModDate" — a reason to prefer
§14.3.2's stream, not a reason to refuse a write to a dictionary the file already carries. And
§14.3.4 is about exactly this edit: where a document states both sources, a processor writing a
date "shall ensure that the data in the document information dictionary and the document level
metadata stream — **if both are written** — are fully equivalent". This face writes one, because
`meta/xmp.xml` is a derived file it refuses to write, so the clause's `shall` is not engaged — but
the inconsistency it is about is now possible, and the write names it in a warning rather than
creating it in silence.

### 6. `attachments/` files in the name tree and not on a page, deliberately

§7.11.4.1 gives an embedded file more than one home, and §12.5.6.15 is the other: a file may be
carried by an annotation on a page. A directory has no page and no rectangle, so a `cp` into
`attachments/` files in §7.7.4's tree, and the annotation home stays the viewer's and the command
line's — where a person can say *which page* and *where on it*. The choice is a permissions
decision as well as a placement one, because Table 22's bit differs with the home (session 885:
bit 6 for a file on a page, bit 4 for one in the tree), and making it silently would have been
making that decision silently.

## Consequences

- A face has nothing left to decide about a write: `create`, `write_at`, `flush`, `release` are
  `open`, `write`, `flush` and `release` with the kernel's own names, and `errno` is a method.
- `Vfs::write` is all four in one call, which is what a KIO `put` is — RFC 0003 §5.4: "KIO's verb
  is already transactional".
- One defect the round found and the shape of it is worth keeping. `crates/pdf-vfs/src/wire.rs`
  says "[n]othing is dropped in silence", and its exhaustiveness is on the **enums**: a `match`
  over `Answer` will not compile with a variant missing. The decode side matches a *byte* and has
  a catch-all by construction, so the round shipped an answer that was encodable and not
  decodable, and every write across the confinement failed with "an answer's kind: 6 is not a kind
  this build defines". What caught it was `tests/confined.rs` asking both workers every question
  and comparing — a both-ways comparison is what turns a one-sided protocol change into a failure
  rather than a silence, and the write queries are in that list now for the same reason.
