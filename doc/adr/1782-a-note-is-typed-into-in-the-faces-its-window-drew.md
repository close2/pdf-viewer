# 1782 — A note is typed into in the faces its window drew, and a timer that changes nothing leaves the page

Session 1469. Status: **accepted** and **built**. Takes back the cost ADR 1770 section 3 named, the
Qt half ADR 1770 section 2 kept, and the timers' site ADR 1771 kept. Context: ISO 32000-2 §12.5.6.14
(the popup "shall be used for editing the parent's text"), §12.5.6.2's Table 172 `/RC` and its
NOTE 1; ADRs 1635, 1721, 1726, 1762, 1770, 1771.
Code: `crates/viewer-host/src/popup.rs` (`run_spans`, `RunSpan`, `editor_not_drawn`),
`crates/viewer-gtk/src/host.rs` (`RunFace`, `run_face`, `run_tag`, `NoteStyle`, `edit_note`),
`crates/viewer-qt/src/host.rs` (`note_spans`, `editor_position`, `contents_byte`, `note_editing`),
`crates/viewer-qt/src/bridge.rs` (`QtNoteSpan`), `crates/viewer-qt/cpp/window.cpp` (`runFormat`,
`NoteView`, `NoteFaces`, `MainWindow::styleNote`, `editNote`), `crates/pdf-model/src/view/script_timers.rs`
(`run_timers`), `crates/viewer-core/src/viewer.rs` (`run_timers`).
Tests: `viewer-host`'s `a_rich_note_s_runs_stand_on_their_contents_bytes` and
`a_rich_note_s_editor_says_what_is_its_paragraphs`; `viewer-qt`'s
`a_note_s_bytes_and_its_editor_s_positions_are_one_place`; `viewer-core/tests/script_timers.rs`
`a_timer_that_changes_nothing_drawn_does_not_interpret_the_page_again`; drive steps 64, 67, 72.

## 1. What the editor holds, and what it sets over it

The text a person retypes is `/Contents` (ADR 1721), and the window draws `/RC` only where its
characters are `/Contents`' (ADR 1635). So the editor keeps holding `/Contents` and sets each run's
face over the characters of `/Contents` that run is: `viewer_host::popup::run_spans` reads the spans
off `rich_offsets`' one alignment (ADR 1770), so the three windows agree about where a run stands.
The note's own `QTextDocument`, which the brief suggested Qt type into, holds `/RC`'s resolved text —
a list tag, chapter 27's collapsed white space — and a save of it would write that over `/Contents`;
it is the window's, and the editor is built beside it.

- **The faces are set again from nothing whenever the window's answer changes**: a run's characters
  move as a person types, and a character that makes `/Contents` disagree with `/RC` makes the window
  plain, so the editor goes plain with it — what is typed into is what the window will show.
- **GTK sets a buffer tag per run** from `RunFace`, the numbers the label's Pango span is written
  from, so the two draw a run alike; **Qt sets them through a `QSyntaxHighlighter`**, whose formats
  are the layout's, so no character changes and nothing is a step a person undoes. A change that is a
  format's alone sends no edit in either (the text last sent is compared).
- **A paragraph's properties are the window's and not the editor's**: a list tag is no character of
  `/Contents`, and an indent, an alignment and tab stops would be set on blocks the person is
  splitting and joining. `editor_not_drawn` says each one the note states under the editor, beside
  what the label already says; Pango sets no glyph scale on a tag either, so GTK's editor says a font
  scale as its label does — the brief's hypothesis held.

## 2. Qt's plain note

A plain window was a `QLabel`, which answers no point for a character, so its editor started at the
end. It is a `NoteView` now — the rich note's widget, over a plain document built by `setPlainText` —
so a press is hit-tested by the document's layout, and its position is the editor's own, whose
document is built from the same text. Every window says where the caret starts as the byte of
`/Contents`, as `quorra` does; Qt's positions count a carriage return and line feed as one block
separator, so `editor_position` and `contents_byte` convert, the rich note's place included.

## 3. The timers

`run_timers` answers `ScriptsRan` as the event and page runners do (ADR 1762), its `changed` the
`Applied::drawn` of each due expression, and the core marks the page stale on `changed` alone.

## Consequences

- Drive step 64 counts the red run's pixels in the editor after the press (red while typed into)
  and after `|` (none: plain); step 67 reads the caret's place in all three windows; step 72 counts
  `quorra`'s renders after two logging timer runs (0) and after a writing one (at least 1).
- `event.keyDown` still reads false, and reporting the arrows from the windows would not change it:
  `ViewState::set_field` raises no Table 199 `/K` for `Entered::Chosen` — which the table makes a
  `shall` where the user "modifies the selection in a scrollable list box" — and `quorra`'s choice
  list takes no arrow key. Those two come first; the windows' report is the last of three.
