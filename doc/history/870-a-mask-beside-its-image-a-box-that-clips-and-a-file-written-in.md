# 870 — A mask beside its image, a box that clips, and the first file written into a document

2026-09-01. Argued in [ADR 0802](../adr/0802-a-mask-is-an-image-on-its-own-grid-a-box-is-a-clip-and-the-first-file-written-into-a-document.md).
The third implementation round of [RFC 0002](../rfc/0002-the-transform-suite.md), on the
long-lived branch `round-867`, which started by merging `main` (rounds 864 to 866 and the
branch's own earlier merge) cleanly.

Touched: `crates/pdf-transform/src/lib.rs`, `src/images.rs`, `src/render.rs`,
`src/attachments.rs`, `src/bin/pdf-transform.rs`, `Cargo.toml` (the `md-5` line),
`tests/verbs.rs`, `tests/gate.rs` (the new plan fields), `tests/writer.rs` (new);
`doc/conformance/ledger.toml` (nine rows extended); `doc/state-of-play.md`,
`doc/todo/README.md`, `doc/todo/57-the-transform-suite.md`; `doc/adr/0802-…` (new), this file.

## 1. What landed

All three items of the round's scope, each decided from the standard before it was written:

- **`images --no-mask`**, and a mask beside the image *always* under `--native`: §8.9.6.1's
  inventory of masks says which of them are images (§8.9.6.3's explicit mask, §11.6.5.2's soft
  mask) and §8.9.6.3 says what such an image is to its base — the same unit square, its own
  resolution — so the mask is written as an 8-bit grey PNG on its own grid, the base decoded as
  if it stated no mask. A JPEG's `/SMask` is no longer dropped. The test derives the relation
  between composite, base and mask from the clauses and holds all three files to it.
- **`render --page-box` and `--no-annotations`**: Table 31's five boxes with their chained
  defaults and §14.11.2.1's intersection, selected through `Page::boundary`; the box asked for
  is both the extent and the clip, a choice argued against §12.2's `/ViewArea`/`/ViewClip`.
  Annotations off is the page interpreted without its `/Annots`. **No first-row crate changed**:
  a `Page` is a value with public fields, and `render` states the page it wants drawn.
- **`attachments --attach FILE [--name] [--description] [--date]`** on §7.5.6's incremental
  update, through `pdf_syntax::write::incremental_update` — three new objects (the embedded
  file stream with Table 45's size and checksum from the bytes, the indirect file specification,
  a one-node `/EmbeddedFiles` root holding every old entry as stated) and one rewritten holder.
  No date unless `--date` is given; the same plan is the same bytes. Table 22 **bit 4** governs
  it — the round was told the table had no bit for attaching, and it has no bit that names it
  and one that binds it.

## 2. What was looked at

Trap 1, for each: the composite, base and mask of `issue21570.pdf`'s striped JPEG side by side;
page 962 of ISO 32000-2 with and without its errata annotations; the appended section of an
attached fixture read as text, and `qpdf --check` and `mutool info` on it.

## 3. Gates

The six core lines, `cargo test -p conformance` and the transform gate were run in this worktree
at `-j 12`; their results are in the round's report and not here (`doc/todo/02` §2's rule). No
first-row crate changed, so the corpus gates were not owed.

## 4. What the next transform round does first

`doc/todo/57`'s order, as rewritten this round: the serializer and the four verbs on it wait on
RFC 0002 §13 question 1; before that, the two first-row items in §3 — `Operation` into
`pdf_model::restriction`, now three variants — are the natural content of a round that runs the
whole gate sequence anyway.
