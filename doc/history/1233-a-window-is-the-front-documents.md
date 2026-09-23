# 1233 — A window is the front document's, and a transition keeps the window's ground

The HOST-UI round of batch thirty-six. ADR 1303. No ledger row moved; none was briefed.

## Driven under Xvfb, two documents each time

- **Qt's submission**, against a loopback server that logs the request and answers in FDF. The
  ask dialogue was answered with `windowfocus --sync` then `key --window`, and the FDF was
  imported with its `/Status` shown. `refuse` then declined the next submission, and nothing
  reached the server. The drive found that the third menu group was missing from Qt's bar,
  because the headings came from `Scope::ALL`. The level could not be set in Qt. It also found
  that every Qt question was titled *Restricted*.
- **The four new styles** in `quorra`, `quorra-gtk` and `quorra-qt`, with mid-transition
  captures. `Blinds /Dm /V`, `Dissolve`, `Glitter /Di 315` and `Fly /Di 0 /B true` all animate
  as Table 164 describes. The drive found a white surround on every frame in all three windows;
  the faces were rasterised as pages. It found that a second document arriving took a running
  presentation's full screen in all three: its `/PageMode` was obeyed while it was behind. It
  found the tab strip left up in full screen, where Right in GTK also changed the document. And
  it found that Qt's first transition started from the behind document's page.
- **The `/Thumb` sentence.** The busy bind belongs to the list being replaced as the view moves
  between notebook pages. No `/Thumb` a reader sees was lost. The row is now asked for again from
  the idle queue, and the sentence is a trace line. A `UseThumbs` tab arriving behind now opens
  its pages panel when it first comes to the front.
- **`doc/todo/38`'s first remaining item**, a pointer on `quorra`'s menu, is built. The question
  card and the menu are now modal to the pointer. Before, a press behind the question could press
  the submit button again.

## Tests

`every_group_the_rows_open_has_a_heading`, `a_face_keeps_the_windows_surround_outside_the_page`,
`a_page_stating_a_thumb_gives_its_row_the_image` (a Table 31 `/Thumb` fixture), and
`a_press_takes_the_level_drawn_under_it`. The per-tab page mode, the hidden strip and the modal
pointer live in each window's event handling, which no gate drives. ADR 1240's cost again.

## Left

- GTK's first transition after launch is drawn at the pre-full-screen size (ADR 1303).
- A stray "copy: off" happened on a Qt menu-bar click and was not reproduced reliably.
- A submission's FDF answer is still worded "import-data:".
