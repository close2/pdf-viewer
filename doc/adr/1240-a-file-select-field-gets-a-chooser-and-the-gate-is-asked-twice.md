# 1240 — A file-select field gets a chooser, and the gate for it is asked twice on purpose

Status: **accepted**.
Context: `crates/viewer-host/src/policy.rs` (`may_choose_file`),
`crates/viewer-host/src/form.rs` (`edit_of`), `crates/viewer-gtk/src/controls.rs` (`entry`,
`offer_a_chooser`), `crates/viewer-qt/src/bridge.rs` (`QtControl::choose_file`),
`crates/viewer-qt/src/host.rs` (`controls`), `crates/viewer-qt/cpp/window.cpp`,
`crates/viewer-host/tests/host_mappings.rs`, `doc/conformance/ledger.toml` §12.7.5.3.
Builds: ADR 1216 (a file-select control's file, and the chooser it said was still owed), ADR 1079
(the policy in one function), ADR 0246, ADR 0526, `doc/ui-boundary.md` rules 2 and 5,
`doc/todo/38`.
Clauses: ISO 32000-2 §12.7.5.3 (Table 231 bit 21), §12.5.2, §12.7.4.3.

ADR 1216 ended by naming what was left: "[n]o window opens a file *chooser*: what a person does is
type the pathname the clause already says the field's text is. A `FileChooserNative` and a
`QFileDialog` are a convenience over that, and the round that adds them can drive a dialogue." This
is that round, and the thing worth not re-litigating is not the dialogue.

## 1. `may_choose_file` existed in a doc comment and nowhere else

`viewer_core::Edit::ChooseFile`'s own documentation said "[a] host asks
`viewer_host::policy::may_choose_file` before it opens the chooser, so the four levels of
`CLAUDE.md` principle 3 attach in one place (ADR 1216)". No such function had ever been written.
That is the sentence-outliving-its-code shape, and the cheap fix would have been to delete the
sentence. The function is the better one, because the sentence describes the right design:
`doc/todo/38`'s *ask* and *warn* levels have to attach somewhere, and a condition written into three
windows is three places.

## 2. The gate is asked twice, and that is the decision

`may_choose_file` answers one question — may this control have a chooser? — from Table 231 bit 21,
which states the whole condition: a file-select control's text "represents the pathname of a file
whose contents shall be submitted as the field's value". It is asked at **two** points on one path:

- by the window, deciding whether to put the affordance on the control at all;
- by `viewer_host::form::edit_of`, when a path comes back, deciding whether the text is a pathname
  or a value.

Asking twice is not redundancy. `edit_of` already had this condition written as a pattern match on
the flag, and a window that decided the same thing for itself would be a second reading of one
clause: the day the condition changes — a level that says *ask first*, a restriction that forbids
it — one of the two would move and the other would not, and the window would offer a chooser whose
result it then refused. A person asked for a file this program will not use has been asked for
nothing. So `edit_of` was rewritten to ask the gate rather than match the flag, and the test
`only_a_file_select_control_offers_a_chooser` holds the two answers together for the same argument.

## 3. The chooser fills the entry in and decides nothing

Neither window builds an edit. The chooser sets the entry's text; the signal the entry already has
fires; `edit_of` turns it into `Edit::ChooseFile` by the route a typed path already took. So
everything ADR 1216 settled about *when* the file is read — once, when the person finishes with the
field, never per keystroke — is untouched, and a chooser is a way of spelling a path rather than a
second way of setting a field.

**The affordance is inside the widget's own rectangle, and that is trap 19's rule rather than a
matter of taste.** §12.5.2's `/Rect` is the *document*'s statement about where the control is, so a
button placed beside the entry would be this window enlarging something the file sized, and the
enlargement feeds back into the geometry the core is told about. GTK gets a secondary
`gtk4::EntryIconPosition` icon on the entry; Qt gets a trailing `QAction` inside the `QLineEdit`.
Both are the platform's own idiom for exactly this, which is trap 17's point about asking whether a
toolkit will *compose* what a clause wants rather than reading a widget catalogue.

`gtk4::FileDialog` rather than `FileChooserNative`, which GTK 4.10 deprecates and this workspace
builds with `-D warnings`.

## 4. The third window is not short of anything

`quorra` has no dialogue toolkit — it draws its own chrome — and the pathname is typed into the field
on the page and committed when the person leaves it. That is not a gap in §12.7.5.3: the clause says
the field's text **is** the pathname, so typing it is the control the clause describes and a chooser
is a convenience over it. A round that gives that window a drawn file browser is welcome to; nothing
in the clause asks for one.

## Cost

Two windows now open a dialogue this machine cannot drive in a test, so what is tested is the seam in
front of it and not the dialogue: `host_mappings.rs` drives `may_choose_file` and `edit_of` with no
toolkit linked, and the two hosts are held level by compiling. A round that adds a third chooser owes
the same gate call, and the day `doc/todo/38` gives it levels, that is where they go.
