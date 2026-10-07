# 1592 — A value is committed from every window: at `/Bl`, at Enter, and a refusal is said

Session 1378. Status: **accepted** and **built**. Builds on ADR 1579 (Tier 0's dispatch and
`ViewState::commit_field`); amends ADR 0713's scope for `quorra-confined` by the keyboard's share of
a form, section 4.
Context: ISO 32000-2 §12.6.3 (Tables 197 and 199), §12.7.4.3, Table 231 bit 13; Adobe's *JavaScript
for Acrobat API Reference*, "Form event processing", the working source the owner named for event
order.
Code: `crates/viewer-core/src/viewer.rs` (`raise`, `commit_typed`, `field_of_widget`,
`refused_keystroke`), `crates/viewer-core/src/open.rs` (`Done::CommitField`, `Open::commit_field`),
`crates/viewer-core/src/command.rs` (`Command::CommitField`), `crates/viewer-confined/src/protocol.rs`
(`COMMIT_FIELD`), `crates/viewer-ui/src/bin/quorra/typing.rs` (`single_line`),
`crates/viewer-gtk/src/controls.rs` (`commits_on_enter`, `commits_on_leaving`),
`crates/viewer-qt/cpp/window.cpp` (`commitsWhenFinished`), `crates/viewer-qt/src/host.rs`
(`commit_control`), `crates/viewer-ui/src/bin/quorra-confined/typing.rs`. Tests:
`crates/viewer-core/tests/field_commit.rs`; `tools/drive-windows.sh` step `37-field-commit`.

## 1. Which event is the commit, and which sentence it rests on

Table 197's `/Bl` is performed "when the annotation loses the input focus", and Table 199's `/V`
"when the field's value is changed". The standard does not say when characters typed one at a time
have *changed* the value; Adobe's event order does — keystrokes, then the keystroke with `willCommit`
set, then validate, calculate, format — and sets `willCommit` when the field loses the focus or Enter
is pressed. So the commit is built on `/Bl`'s sentence: **wherever `viewer-core` moves the focus off a
widget** — a tab, a press elsewhere, a page turned — `Viewer::raise` commits that widget's field
before it performs the widget's `/Bl` action. Enter is the second: Table 231 bit 13 clear says the
text "shall be restricted to a single line", so Enter in a single-line text field (or a combo box's
edit) enters no character, and a host sends `Command::CommitField`. The same message is what a
toolkit host sends when its own control loses the keyboard without `viewer-core`'s focus moving —
a click into a `GtkEntry` the core never saw. A second commit of one field finds nothing typed and
does nothing, so the two routes never double a refusal.

## 2. The commit is an entry of the log

Undo and redo are replays of the log (`Open::replay`), so a commit outside it would come undone at
the next edit to any field. `Done::CommitField` is logged when `ViewState::commit_field` answers other
than `Nothing`, and replayed by calling it again: its outcome is a function of the entries before it.
An undo of it leaves the field typed and uncommitted, which is what the person had before pressing
the key.

## 3. A refusal is a sentence, at the commit and at a keystroke

`Committed::Refused`'s sentence goes out as `Event::Reported` with no page — the channel every window
already shows (quorra's `note:` line, the toolkit status bars, the confined window's standard error).
A character Table 199's `/K` refuses in its typing form — "[t]his action may check the added text
for validity and reject or modify it" — changes nothing, and `aform` refuses it in silence because
Adobe's library raises no alert there; Adobe *beeps*. This program says it instead, once per refused
keystroke, and logs nothing: an entry that changed nothing would be an undo step over nothing. The
judgement is the model's — the edit is tried on a copy of the view, and only where no widget took it
and the field's one-call keystroke script rejects the characters is the sentence said, so a read-only
field whose script would take them is not reported as the script's refusal.

## 4. The confined window types

ADR 0713 drew `quorra-confined` with no form controls. The commit runs where the scripts are, inside
the confinement, and a boundary nobody can type across has not been shown to carry it; so the window
takes the keyboard's share: Tab and Shift-Tab walk §12.5.1's order, a single-line text field the walk
lands on takes characters (each keystroke reads the worker's value back and sends the whole, as the
flagship does under ADR 0197), Backspace, Enter commits, Escape leaves the field (the page's Escape is
still the abort), Control and S saves beside the file. Nothing is drawn over the page.

## 5. What is not done

The two toolkit windows show a committed value in their own controls as typed — `12.5`, where
`quorra` draws `$12.50` — because `Query::Fields` carries the value and not what `/F` makes of it;
the saved file and the page `quorra` draws carry the format. Carrying the displayed value beside the
value, and writing it into a control that does not have the keyboard, is the remainder, named in
`doc/todo/56`.
