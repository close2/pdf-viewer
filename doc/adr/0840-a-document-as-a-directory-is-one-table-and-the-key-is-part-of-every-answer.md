# 0840 — A document as a directory is one table, and the generation key is part of every answer

Session 899. Status: **accepted**. The first decision record of RFC 0003's implementation, and the
first landing of the file-system faces the owner approved on 2026-09-03 ("RFC 002 and 003 are
approved").

## Context

RFC 0003 proposes two frontends — a KIO worker so Dolphin browses into a PDF the way it browses
into a tar, and a FUSE filesystem so every program on the machine sees the same tree — over **one**
core, `pdf-vfs`. Its §7 is explicit about what the core owns and what the faces may not: the
layout as one declarative table, the generation cache, the generation-key consistency rules, and
the broker side of the confined-worker protocol, with "[t]he faces contain *no* layout knowledge —
adding `fonts/` one day is a core change that both faces grow simultaneously".

The owner sequenced this stream **after** RFC 0002's writing verbs, for the reason `doc/todo/57`
§2 records: running them in parallel would have had both implementing the same things. Six verbs
of the transform suite have landed since, one of which writes whole files, so what this round
consumes is a seam that has survived contact rather than a proposal.

This round is the core and the **read side only**. No FUSE binary, no KIO plugin, no C ABI: each
carries its own toolchain risk (the RFC rates FUSE low and KIO moderate), and a face written over
an unproven core would have to be rewritten when the core moved.

## Decision

### 1. The layout is a table, not a `match`

`pdf_vfs::layout::LAYOUT` is a `static` array of `Route`s, one per shape of path, each stating
four things: the pattern, whether it is a directory or a file, **what generates it**, and **what a
write to it would mean**. `path::resolve` is the only code in the crate that looks at a path's
text, and it answers with a row and what the path captured.

A `match` over path components would have answered "what is this path" and nothing else. The table
also answers, without running anything: what is in the tree, what each entry costs, which clause
each generator rests on, what each file verb would do, and — by subtraction — which of those verbs
is declared and unbuilt. Three consumers need exactly that population and each of them would
otherwise have written its own: `Vfs::shortfalls`, `Vfs::write_meaning` (what a KIO worker
consults before advertising an operation), and the harness's own coverage test, which walks
`LAYOUT` and asserts that every row resolves from a path that matches its pattern.

### 2. A row has two write meanings, because a file verb is two verbs

The first shape of the table gave each row one `write` field, and it was wrong within an hour of
being written: `cp new.pdf pages/0004.pdf` and `rm pages/0004.pdf` address the *same* row and mean
two different operations — RFC §5.2's page insertion and its page deletion. A single field made
`attachments/NAME` say "removing this embedded file" to a `cp` that was trying to embed one.

So a row states `WriteMapping { on_write, on_delete }`, which is RFC §5.2's own table (a verb
column and a meaning column) as data. It also made a gap visible that a single field had hidden:
**deleting `meta/info.json` is not one of the five verbs the RFC states**, so it gets its own
refusal reason — `NotOneOfTheFiveVerbs` — rather than being quietly folded in with the four
refusals §5.3 argues for.

### 3. Six generators are a `pdf_transform::Plan` and nothing else

RFC §7 requires that the core consume the transform layer for page extraction and the existing
readers for the rest, and that it implement neither. What that means in code:

| the tree | is | and the seam call is |
|---|---|---|
| `pages/NNNN.pdf` | a complete single-page PDF | `Plan::Split`, one page selected |
| `renders/DPI/NNNN.png` | the page drawn | `Plan::Render`, `Sizing::Dpi` |
| `images/NNNN/…` | that page's images | `Plan::Images`, `native` on |
| `attachments/NAME` | §7.11.4's embedded file | `Plan::Attachments`, `Action::Save` |
| `text/NNNN.txt` | the page's readback | `pdf_model::interpret(..).text` |
| `meta/info.json` | §14.3.3's dictionary | `pdf_model::metadata::Information` |
| `meta/xmp.xml` | §14.3.2's stream, decoded | the catalog's `/Metadata` |
| `meta/outline.json` | §12.3.3's outline | `pdf_model::outline::Outline` |

`tests/a_face.rs::a_page_out_of_the_mount_is_the_transform_suites_own_piece` holds the first row
**byte for byte** against `pdf_transform::apply` over the same page, and
`a_pages_text_is_the_interpreters_own_readback_byte_for_byte` holds the fifth against
`interpret`. Those two assertions are what makes RFC §7's prohibition checkable rather than a
promise: if this crate ever grows a second implementation of extraction or of text, they fail.

**One thing the seam had to gain**, and it is three lines: `pdf_transform::Source::document`.
Three generators are readers no verb covers, and a consumer that opened the document itself would
need its own copy of §7.6.4.1's password — while `viewer_core::Secret` is deliberately not
`Clone`, precisely so that a second buffer does not exist. One `Source` now holds the password and
both the verbs and the readers reach the document through it.

### 4. The generation key is asked before every answer, and it is the cache's key

RFC §5.4: "[e]very operation validates the key before answering; a changed key rebuilds the
virtual tree." Every public method of `Vfs` begins with `current()`, which asks the backing for
`Generation { modified_nanos, size, startxref }` and, on any difference, throws away the worker,
the inventories and **every cached output of the old generation** before answering anything.

The third component is §7.5.5's own offset, and it is there because the first two are not enough:
a file system's timestamp granularity is coarser than an edit, and an incremental update that
replaces one object with another of the same length changes neither number. Reading it is a
bounded scan of the last 4096 bytes for an ASCII keyword — not a parse, which matters because this
runs on the **broker's** side of §6's boundary, where no PDF may be parsed.

Two properties follow and both are tested against a backing the test replaces under the tree:

- `the_document_changing_under_the_mount_rebuilds_the_tree` — the page count, the listing and the
  bytes of `pages/0001.pdf` all become the new document's, and the old page is not served.
- `an_open_file_keeps_the_generation_it_was_opened_under` — a `Handle` holds its bytes and its key,
  so RFC §5.4's "[n]o reader ever receives a splice of two generations" is a property of the
  type's *shape* rather than of a check somebody remembered to write.

A third, `a_page_the_new_generation_does_not_have_stops_being_a_path`, is the one a face would hit
first: a document that got shorter makes `pages/0009.pdf` `NoSuchPath`, not a stale page.

### 5. `stat` generates, because an estimated size truncates the file

RFC §5.5 states the rule and §2 states the evidence — ffmpegfs and mp3fs both document it: the
kernel clamps reads at the size `stat` reported, so an under-estimate silently truncates the file
for every reader. So `Vfs::stat` on a file opens it, and the cache is what stops a `cp` — which is
a `stat` and then a `read` — paying twice.

The cache is content-addressed by (generation key, path) with one explicit ceiling in bytes,
evicting least-recently-used entries until the new one fits. An entry larger than the whole budget
is **answered and not stored**: refusing would make the budget a limit on what the mount can serve
rather than on what it remembers, and the guard against a decompression bomb is
`pdf_syntax::Limits` inside the worker where the bytes are (`doc/todo/10`'s distinction).

### 6. Every refusal is loud, and the two kinds are told apart

`Refused` has two variants and the difference is the design being visible from outside:

- **`ByDesign`** — RFC §5.3's four, plus the two this round added (a directory is the document's
  shape; deleting `info.json` is not one of the five verbs). Each carries a `Reason` whose
  `sentence()` says *why it will still be refused when the write side lands* — a rename inside
  `pages/` because position-names make a reorder ambiguous, a write into `text/` because RFC 0005
  owns that with a caret and a font answer, a write into `images/` because nobody has designed it,
  a write into `renders/` or `meta/xmp.xml` because they are derived.
- **`NotYetImplemented`** — the layout knows what this write means and RFC 0003's write side has
  not landed. It names the operation ("inserting the copied document's pages at this position")
  and points at the `pdf-transform` command line.

A face that could not tell them apart would report "read-only file system" for both. The sentences
live in the core rather than in either face because FUSE has no message channel — §5.3's own
observation, and its reason for insisting the sentence exist at all.

## Consequences

- Two faces can now be written with no layout knowledge in either. `tests/a_face.rs` is the core
  driven the way a face drives it — `listDir`, `stat`, `get`, plus the `open`/`read` split a mount
  needs — which is exactly `kio_archive`'s read-only surface.
- **`ls images/` is per page**, which departs from RFC §4's flat directory. ADR 0841 §3 has the
  argument; it is a departure from an approved document and `doc/todo/58` carries it for the owner
  to overrule.
- The write side is one round away and its shape is fixed: each row already states what the verb
  means, so the work is the transform calls and the commit point, not a design.
- What the round does not do is named by `Vfs::shortfalls` rather than left to be discovered: a
  §12.3.5 collection listed flat, `document.txt` built whole rather than streamed, no disk half to
  the cache, and an encrypted document that opens only under §7.6.4.1's default user password —
  that last for a reason worth keeping, that a worker is made per generation and a `Secret` cannot
  be copied, so a mount that survives a change of the file needs a design for re-supplying it.
