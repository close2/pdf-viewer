# 885 — A file attached is an edit, and the four levels reach a window

2026-09-03. Argued in [ADR 0814](../adr/0814-a-file-attached-is-an-edit-the-four-levels-reach-a-window-and-the-writer-moves-beside-its-reader.md).
On the worktree branch `round-885`, from the owner's *"I am missing adding embedded files"*.

**This round was interrupted and finished by a different model.** The first agent was cut off
by a rate limit partway through, with the whole design written and uncommitted in the worktree
and its last note reading *"Now the tests: the model's own round trip, and the viewer's four
levels for both homes"* — which it had in fact already written. A second agent took the design
over, checked it against the clauses, fixed what the gates found, wrote the ledger rows and the
documents, and committed. The scope, the argument and the code are the first agent's; the
clause-by-clause check, the four lint findings below, the ledger, `doc/todo/38`,
`doc/ui-boundary.md`, `doc/state-of-play.md` and this file are the second's.

Touched: `crates/pdf-model/src/attachment/filing.rs` (new — the shared writer),
`crates/pdf-model/src/attachment.rs`, `crates/pdf-model/src/view.rs` (`Filing`, `FilingHome`,
`Filed`, `attach`, `detach`, `attachments`, `write_filings`, `allocate`);
`crates/pdf-transform/src/attachments.rs` and its `Cargo.toml` (the verb over the moved writer,
`md-5` gone with the digest); `crates/viewer-core/src/{command.rs,event.rs,lib.rs,notes.rs,
open.rs,viewer.rs}`; `crates/viewer-confined/src/protocol.rs`; `crates/viewer-ffi/` (three entry
points, three event kinds, four constants, and its three self-checking tests);
`crates/viewer-gtk/src/host.rs`, `crates/viewer-qt/src/host.rs`, `crates/viewer-ui/src/bin/`;
`crates/viewer-host/src/policy.rs`; tests in `pdf-model` (new `tests/attachments_filed.rs`) and
`viewer-core` (`tests/headless.rs`); `doc/conformance/ledger.toml`; `doc/todo/38-…`,
`doc/ui-boundary.md`, `doc/state-of-play.md`; `doc/adr/0814-…` (new), this file.

## 1. What landed

The whole of the round's scope, and no gesture:

- **Attach and detach as edits in the log beside the immutable document.** `Edit::Attach
  { bytes, name, description, mime, home }` and `Edit::Detach { name }`, resolved into
  `Done::Attach`/`Done::Detach`, replayed by undo and redo, written by §7.5.6's incremental
  update at `Command::Save` and at no other time. `pdf_syntax::Document` is untouched.
- **One writer, moved.** `pdf_model::attachment::filing` builds §7.11.4's stream, §7.11.3's
  specification, §12.5.6.15's annotation and §7.7.4's tree; `pdf-transform` and `ViewState::save`
  are two allocators over it. The crate graph runs `pdf-transform → viewer-core`, so the shared
  writer could not live in the transform.
- **All four levels, each with the message that answers it.** `Event::Asking` +
  `Command::Answer`, `Event::Warned` after the `Dirty`, `Event::Refused` as the answer of exactly
  one level, and `Off` proceeding. `Event::AttachmentsChanged` is the fourth message.
- **The shared Files tab lists the log's view**, so a just-attached file is there before any save
  and a detached one is gone at once.
- **The edit crosses the confinement whole**, bytes on the wire as `Command::Open`'s are. Round
  883's `SCM_RIGHTS` route for a *document* was not on `main` when this branched; the route is
  recorded as owed in `doc/todo/38` rather than built against a branch that does not exist.

## 2. The bit the round had to decide

Which of §7.6.4.2's Table 22 bits governs a file attached *on a page*. Settled from the table's
own words and §12.5.6.15: **bit 6**, "Add or modify text annotations…", because that clause makes
the file part of the annotation and bit 4's own row hands whatever bit 6 controls to bit 6. The
tree stays bit 4's residual (ADR 0802). The argument, and the two readings refused, are in ADR
0814 and on the §7.6.4.2 ledger row; the consequence is pinned by a test — a certification at
§12.8.2.2's level 3 admits a file on a page and withholds one in the tree.

## 3. What the gates found

Four `clippy::pedantic` findings, all `too_many_lines` or `match_same_arms` caused by the new
arms, and each fixed by an extraction rather than an `#[expect]` (this tree has none for
`too_many_lines`): `password_required` out of the confined window's `event`, `extracted` and
`restricted` out of `Events::describe`, `page_changed` out of `dispatch::react`, and the
four-level test split into one function per level. And **four `viewer-ffi` self-checks**, which
are the ABI's own protection working exactly as designed: the header's constants against the
library's, the entry-point count in two files, and the C driver's expected line — every one a
number a person has to write beside the change, in the same commit.

## 4. What the gestures still owe

A drop, a dialog or a palette entry that sends `Edit::Attach`; a row action that sends
`Edit::Detach`; a prompt for `Event::Asking`; and a way to *choose* `Ask` or `Warn` at all, since
a command line is not one. `doc/todo/38` holds the list, and the owner's mockups decide the flows.
