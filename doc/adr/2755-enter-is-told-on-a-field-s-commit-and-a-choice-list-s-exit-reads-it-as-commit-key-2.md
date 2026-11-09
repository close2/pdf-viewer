# 2755 — Enter is told on a field's commit, and a choice list's `exit` reads it as `commitKey` 2

Session 1972, a ledger round (K.2). Status: implemented. Extends ADR 2726 (`commitKey` 1 and 3 at
this crate's own focus moves) and ADR 2707 (a choice list's selection commit fires its `exit`).

## Context

XFA 3.3 page 402 defines `$event.commitKey` as what "happened to the value in the field when the
form field lost focus", with four values: 0 not committed (Escape), 1 a click of the mouse outside
the field, 2 the Enter key, 3 a tab to a new field. Page 398's table has a field's `exit` set it.
ADR 2726 numbered 1 and 3, the two that `viewer-core` sees itself (a press elsewhere, a tab).
Page 392 has a choice list's selection commit fire the list's `exit` though the focus stays, and
that commit reaches `viewer-core` as `Command::CommitField`, which every host sent alike for Enter
and for a click on an item — so the `exit` could say nothing, and refused the read by name.

## Decision

1. **`Command::CommitField` carries `by: Committing`**, `Enter` or `Unstated`. Each host says
   `Enter` where its toolkit tells it the key: `quorra` and `quorra-confined` commit only on Enter;
   `viewer-gtk` already told the two apart (`FieldChange::Commit::enter`, the entry's `activate`);
   `viewer-qt` notes `returnPressed`, which Qt emits for the key before `editingFinished`; the WinUI
   host commits on Enter in its `KeyDown` and its combo box's `TextSubmitted` before the page takes
   the keyboard; `viewer-ffi` adds `quorra_commit_field_on_enter` and
   `quorra_window_commit_field_on_enter`, leaving the existing two `Unstated`. The confined
   protocol writes the flag as a byte after the field's name.
2. **A choice list's selection `exit` reads Enter as 2** (`Stimulus::Selected::committed`), and a
   commit the host cannot place leaves the property unoffered, refused by name as ADR 2726's untold
   case is. A click on an item is within the field and so none of page 402's "click of the mouse
   outside the field": a 1 there would be a reading the page does not state.
3. **An interactive form field's `event.commitKey` is unchanged.** `ViewState::commit_field` keeps
   ADR 1626's documented `Enter` for a host that does not say, so nothing an AcroForm script reads
   moves.

## Consequences

`pdf_model::xfa::Exited` names the four values together (ADR 2756 adds the 0). Four entry points
move `unsafe_position.rs`'s and `header_and_library_agree.rs`'s counts; `QUORRA_ABI_VERSION` stays.
