# 872 — Every bit named, the four levels in one place, and a file on a page or taken out

2026-09-02. Argued in [ADR 0803](../adr/0803-every-bit-named-the-four-levels-in-one-place-and-a-file-on-a-page-or-taken-out.md).
The fourth implementation round of [RFC 0002](../rfc/0002-the-transform-suite.md), on the
long-lived branch `round-867`, which started by merging `main` (round 869's raster-cache fix)
cleanly.

Touched: `crates/pdf-model/src/restriction.rs` (rewritten: `Bit`, five `Operation`s, `Level`,
`Verdict`, `decide`); `crates/pdf-syntax/src/write.rs` (`incremental_update_freeing`, free
entries in both section forms); `crates/pdf-transform/src/lib.rs`, `src/attachments.rs`,
`src/bin/pdf-transform.rs`; `crates/viewer-core/src/command.rs`, `src/viewer.rs`,
`src/notes.rs`; `crates/viewer-confined/src/protocol.rs`; tests in `pdf-model`, `pdf-syntax`
and `pdf-transform`; `doc/conformance/ledger.toml`; `doc/state-of-play.md`,
`doc/todo/README.md`, `doc/todo/38-…`, `doc/todo/57-…`; `doc/adr/0803-…` (new), this file.

## 1. What landed

All three items of the round's scope:

- **The restriction policy, once, in `pdf-model`.** Every Table 22 position named with its
  clause, two stated as consumed by nothing; the transform's three operations moved in beside
  the viewer's two, so one module reads Table 22 *and* §12.8.2.2's certification for every
  operation this tree performs; the four levels as `Level` and the answer as an exhaustive
  `Verdict`, asked once by `decide`. `pdf-transform` consumes all four (a pipe's *ask* is its own
  refusal, and the command line refuses the word before opening the file); `viewer-core`
  supplies the two it can answer and matches every verdict.
- **`attachments --attach --to-page N [--rect 'x y w h'] [--icon NAME]`**: §12.5.6.15's
  annotation on the page, `/Annots` rewritten where it is, no `/AP` — the icon is this tree's
  own artwork, and the default placement is a stated choice.
- **`attachments --remove NAME`**: the tree without the entry, and the specification and
  stream marked free by §7.5.4's second mechanism, because an update's subsections "can never
  have an object number of zero" and so cannot re-head the linked list. The bytes stay.

## 2. What was looked at

Trap 1: the `Paperclip` and `Graph` icons on page 1 of `PDF20_AN001-BPC.pdf` at 150 dpi,
cropped, one at a stated rectangle and one at the default; `qpdf --check` on the attached,
page-attached and removed files; `--list` on each.

## 3. Gates

`pdf-model` and `pdf-syntax` changed, so the whole `doc/todo/02` §2 sequence was run in this
worktree at `-j 12`, with the transform gate; the results are in the round's report and not here.

## 4. What the next transform round does first

`doc/todo/57`'s order, as rewritten this round: the serializer and the four verbs on it wait on
RFC 0002 §13 question 1. Before that, and small: `--format pgm` on a stated grey conversion, the
`/Names`-dictionary-indirect holder fixture, and `--password-prompt` behind `doc/stack.md`'s
decision on a terminal-mode dependency.
