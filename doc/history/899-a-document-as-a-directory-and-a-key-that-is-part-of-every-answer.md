# 899 — A document as a directory, and a key that is part of every answer

2026-09-03. Argued in [ADR 0840](../adr/0840-a-document-as-a-directory-is-one-table-and-the-key-is-part-of-every-answer.md)
and [ADR 0841](../adr/0841-the-broker-holds-no-document-and-a-flat-directory-cannot-be-listed.md).
The **first** implementation round of [RFC 0003](../rfc/0003-file-system-faces.md), on the
transform stream's branch because RFC 0003 §7's core consumes RFC 0002's seam; `main` had moved
under round 898's merge of `round-867` and was merged in before the commit.

The owner approved RFC 0003 on 2026-09-03 in the same sentence that approved RFC 0002 — "RFC 002
and 003 are approved" — and sequenced this stream *after* the transform suite's writing verbs, so
that the two would not implement the same things. Six verbs of that suite have landed since. This
round is what the sequencing was for: **the shared core, and the read side only.**

Touched: **`crates/pdf-vfs/`** (new — `lib.rs`, `layout.rs`, `path.rs`, `worker.rs`,
`generation.rs`, `cache.rs`, and `tests/a_face.rs`); `Cargo.toml` and `Cargo.lock` (the workspace
member); `crates/pdf-transform/src/lib.rs` (`Source::document`, three lines);
`doc/conformance/ledger.toml` (four rows gain a consumer); `doc/rfc/0003-…` and `doc/rfc/README.md`
(the status line); `doc/todo/58-the-file-system-faces.md` (new), `doc/todo/README.md`,
`doc/todo/57-…` (the hand-off, taken), `doc/todo/02-every-round.md` (one row of the gate map);
`doc/state-of-play.md`, `doc/crate-map.md`; two ADRs, this file.

## 1. What the round claims, and what holds it

Three sentences, each with an assertion behind it rather than a promise:

- **`cp` *is* page extraction.** `pages/0002.pdf` is byte for byte what `pdf_transform::apply`
  writes for the same page, and a test says so. That is RFC §7's prohibition — the core contains
  no PDF logic of its own — made checkable.
- **The mount's text is the extraction identity.** `text/0003.txt` is
  `pdf_model::interpret(..).text` byte for byte, so a caller that greps the mount is grepping the
  bytes `text_extraction.rs` measures against `pdftotext`.
- **No reader ever gets a splice of two documents.** The generation key is asked before every
  answer; an open `Handle` carries its bytes *and* its key, so the property is the type's shape
  rather than a check somebody remembered.

## 2. The one design decision that was wrong for an hour

The layout's first shape gave each row a single `write` field. `cp new.pdf pages/0004.pdf` and
`rm pages/0004.pdf` address the same row and mean two different operations, so
`attachments/NAME` told a `cp` trying to embed a file that a write there meant *removing* one.
A row now states `WriteMapping { on_write, on_delete }`, which is RFC §5.2's own table as data —
and the second field immediately found a gap the first had hidden: **deleting `meta/info.json` is
not one of the five verbs the RFC states**, so it has its own refusal reason rather than being
folded in with the four §5.3 argues for.

## 3. The departure, recorded rather than absorbed

`images/` is a directory per page — `images/0035/01.png` — where RFC §4 draws it flat. ADR 0841 §3
has the argument, and it is the RFC's own §5.1: a flat directory cannot be listed without
extracting every image in the document, because a file form depends on the codec **and** on
whether §8.9.6's mask travels beside it, which is known only after extraction. Predicting the
names instead would make a listing that can name a file a read cannot produce. Per page, the
listing of `images/0035/` and a read out of it are one call and cannot disagree.

It is a departure from a document the owner approved, so `doc/todo/58` §1 carries it for the owner
to overrule or ratify, and the RFC's own status line now names it.

## 4. What was looked at

`doc/PDF-Declarations.pdf`, whose two §7.11.4 embedded files are filed under names holding a
COLON — the witness that made sanitisation a mapping to be tested rather than a line to be
written; `doc/Tagged-PDF-Best-Practice-Guide.pdf`, whose images sit on five of its seventy-two
pages and none on page 1, which is what makes the per-page listing's cheapness visible;
ISO 32000-2 §7.5.5 for why the generation key's third component is the last `startxref` offset and
why reading it is a bounded tail scan rather than a parse; §7.11.4.1, §14.3.2, §14.3.3 and §12.3.3
for the four generators no transform verb covers; and ADR 0812's `SCM_RIGHTS` route, which is why
a worker is handed its document once, as `FileBytes`, at spawn.

## 5. Gates

`pdf-vfs` is new and under no corpus gate; `pdf-transform` gained a public method and no
behaviour, and documents changed. The whole `doc/todo/02` §2 sequence was run in this worktree all
the same, the walking lines under `tools/bounded.sh` and one at a time on the machine, waiting for
a neighbouring round's `pages_corpus` to finish first. The results are in the round's report and
not here.

## 6. What the next round of this stream does first

`doc/todo/58`'s order: **the confined worker before any face**, because a mount is entered by
anything that touches a folder and `InProcess` parses hostile bytes with the caller's privileges;
then the write side, whose meanings the layout table already states; then the FUSE face, which is
the pure-Rust one. And the owner has not been asked RFC §9's seven open questions since approving
the document.
