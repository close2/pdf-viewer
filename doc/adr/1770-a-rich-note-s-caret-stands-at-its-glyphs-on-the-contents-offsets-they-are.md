# 1770 — A rich note's caret stands at its glyphs, on the `/Contents` offsets they are

Session 1464. Status: **accepted** and **built**. Takes back the one case ADR 1739 kept. Context:
ISO 32000-2 §12.5.6.14 (the popup "shall be used for editing the parent's text"), §12.5.6.2's Table
172 `/RC` and its NOTE 1; ADRs 1635, 1721, 1726, 1739.
Code: `crates/viewer-host/src/popup.rs` (`rich_offsets`), `crates/viewer-ui/src/chrome/rich.rs`
(`lay_out`, `placed`, `Placed`), `crates/viewer-ui/src/chrome.rs` (`popup_carets`, `rich_carets`),
`crates/viewer-gtk/src/host.rs` (`note_place`, `PlacedPopup`, `NoteText`),
`crates/viewer-qt/src/host.rs` (`note_place`), `crates/viewer-qt/cpp/window.cpp`
(`RichNoteView::positionAt`, `PopupWindow::mousePressEvent`, `editNote`).
Tests: `viewer-host`'s `a_rich_note_s_places_are_its_contents_offsets`; `viewer-ui/tests/panel.rs`
`a_rich_note_s_caret_stands_at_its_glyphs_and_a_press_there_finds_it` and its control; drive step 64.

## 1. What the caret stands in

The text a person retypes is the note's `/Contents` (ADR 1721), and `pdf_model::popup` shows the
`/RC` only where its words are `/Contents`' words: NOTE 1 expects the two "textually equivalent",
and where they are not the plain text is drawn (ADR 1635). So every character that is not white
space is the same character on both sides in the same order, and the alignment is exact there.
White space is what differs — chapter 27 has resolved the string's, a paragraph is a break
`/Contents` spells as a carriage return or a line feed, a tab is a stop the string counted — and
§12.5.6.6 states no collapse of its own, which the brief named; the collapse is chapter 27's and
`same_characters`'. A white-space run the window draws holds its places one for one with the run
`/Contents` has there, and its last place is the end of that run: ADR 1739's plain rule, one place
each side of a space drawn for several. `viewer_host::popup::rich_offsets` makes that one alignment
for all three windows, and a note whose characters disagree has none.

## 2. Decision

- **`quorra` places the caret at the glyphs `rich::draw` laid out**: the drawing and the places are
  one function, `lay_out`, drawing where a list is given and saying where each glyph went where one
  is not, so the caret cannot stand where the glyph is not. Each glyph has a place at each edge of its
  stored characters — the left then the right read left to right, the right then the left at an odd
  UAX #9 level — and an empty line one at its start; the caret is as tall as the line's tallest em.
  A press, the four arrows, Home and End work as in a plain note; a character typed makes the words
  disagree and the window plain, which is ADR 1721's existing rule.
- **The toolkits' editors start at the character the press was on.** Their text views report a
  glyph's offset, so the brief's hypothesis did not hold; but they retype in a plain text view
  that replaces the window (ADR 1726), so the rich layout is the *label's*, and the press is read
  there before the editor is built: GTK asks the label's Pango layout (`xy_to_index`), for a plain
  note and each rich paragraph alike; Qt asks the rich note's own `QTextDocument` layout
  (`hitTest`), whose positions the host walks back through the runs it handed over. Qt's plain note is
  a `QLabel`, which answers no point for a character, so it starts at the end as before.

## 3. Consequences

- Drive step 64 presses the red run's first letters of `drive-note-file.pdf`, moves Right, Left,
  Left and types `|`: the saved `/Contents` holds it at byte 1 to 4 in every window, where a caret
  left at the end puts it at 9.
- The toolkits' editor still shows the note's characters plain while it is typed into: Table 172's
  formatting is the label's, and a toolkit editor carrying the runs is not built.
