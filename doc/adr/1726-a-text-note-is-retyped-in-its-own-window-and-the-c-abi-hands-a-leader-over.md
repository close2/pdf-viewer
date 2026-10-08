# 1726 — A text note is retyped in its own window, and the C ABI hands a tab leader over

Session 1445. Status: **accepted** and **built** in `viewer-core`, the three windows and the C ABI.
Builds on ADRs 1721 (a text note's text is a reader's edit, `ViewState::set_note_text`), 1723 (a
note with no popup opens its own window), 1720 (a window's state is the later writer's), 0238 (a
free text annotation's retyping, named by object) and 1679, 1722 (a leader drawn in each window).
Context: ISO 32000-2 §12.5.6.14, §12.5.6.4, §12.5.6.2 (Table 172), Table 167 bit 10.
Code: `crates/viewer-core/src/command.rs` (`Edit::SetNoteText`), `query.rs` (`PopupWindow::note`),
`viewer.rs` (`changes_nothing`, the window's note), `open.rs` (`Done::SetNoteText`);
`crates/pdf-model/src/popup.rs` (`retyped_by`, `retypable`); `crates/viewer-host/src/popup.rs`
(`Window::note`); `crates/viewer-ui/src/bin/quorra/typing.rs` (`press_on_note`, `typed_into_note`),
`overlays.rs` (the ring); `crates/viewer-gtk/src/host.rs` (`edit_note`, `end_note`, the press in
`pointer`); `crates/viewer-qt/cpp/window.cpp` (`editNote`, `endNote`), `src/host.rs` (`set_note`,
`note_editing`); `crates/viewer-confined/src/protocol.rs` (edit 7, `PDFVCF11`);
`crates/viewer-ffi/src/abi.rs` (`quorra_popup_note`, `quorra_set_note_text`,
`quorra_popup_rich_leader`, `quorra_popup_rich_leader_text`, `quorra_leader_cycles`,
`quorra_rule_pieces`). Tests: `viewer-core`'s
`a_note_is_retyped_through_its_window_and_saved_as_its_contents`,
`a_window_whose_text_is_not_a_note_s_offers_no_retyping_and_refuses_one`; `viewer-ffi`'s
`a_leader_crosses_with_its_stop_and_is_not_said` and the C program's leader and note lines;
`tools/drive-windows.sh` step 64 in the three windows.

## 1. What was missing

§12.5.6.14: "A popup annotation ( PDF 1.3 ) displays text in a popup window for entry and editing.
It shall not appear alone but is associated with a markup annotation, its parent annotation, and
shall be used for editing the parent's text." The view state held a note's retyping since ADR 1721
and no host command reached it. A host could not name the note either: a window is keyed by its
popup, or by the note for ADR 1723's own window, and a window's text is a text note's in some
windows and a square's or a highlight's in others — whose `Contents` this program does not retype
(ADR 1721).

## 2. The construction

- **`PopupWindow::note`** names the note whose text the window shows and a person may retype:
  Table 186's `/Parent`, or the note itself for its own window, or — for §12.5.6.2's subordinate,
  whose `Contents` is a group attribute — the primary its `/IRT` names. `None` for any other markup
  annotation's window, which a host offers no keyboard to. `pdf_model::popup::retypable` is the one
  predicate, the same `set_note_text` applies.
- **`Edit::SetNoteText { annotation, text }`** carries the whole text, as a field's value is sent,
  and names the note by object for `SetFreeText`'s reason. It is Table 22 bit 6's annotating and is
  read under Table 167 bit 10 like any edit naming an annotation; an edit naming anything but a
  retypable note is said and not logged.
- **In the two toolkit windows a press on the window places a text view over it** — a
  `GtkTextView` in the controls' layer (the windows' layer takes no pointer, so the press is read in
  `pointer`), a `QPlainTextEdit` over the page (the window takes a press only where it retypes) —
  holding the text, the caret at its end, every change sent; Escape gives the keyboard back, and so
  does the window closing. The window's `/RC` formatting is the label's: a retyping is characters,
  and the window draws the note formatted again where they still agree (ADR 1721).
- **In `quorra` the caret stands at the note's end**, and the window's focus ring says it has the
  keyboard. This host lays a window's lines out itself in `chrome::popup_windows`, which answers no
  point for an offset, so a caret placed inside the text would be drawn where the next character
  does not go. Characters, a space and a return are added, Backspace takes the last character, and
  every other key is consumed. **The cost**: a person cannot move the caret into a note in `quorra`;
  the two toolkit windows place one wherever a press lands.
- `quorra-confined` draws no popup window, so it has none to type into; the wire carries the edit
  and the field for a window that does.

## 3. The C ABI's leader

`quorra_popup_rich_tab` handed a stop's position and alignment and not its fill, so a C caller was
told "a tab leader in a popup window" among what is not carried out. The leader now crosses as the
toolkits receive it from `viewer_host::popup::tab_stops`: `quorra_popup_rich_leader` writes its
pattern (none, dots, a rule and its style, content), a rule's thickness and the leader's width in
the units a stop's position crosses in, `quorra_popup_rich_leader_text` the content's characters,
and `quorra_leader_cycles` and `quorra_rule_pieces` the grid the three windows break a leader on, so
that a C caller paints the same repetitions over its own line. Nothing is said for a leader any
more; `viewer_host::popup::leader_unapplied` is gone.
