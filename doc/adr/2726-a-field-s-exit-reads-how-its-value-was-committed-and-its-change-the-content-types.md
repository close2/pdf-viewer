# 2726 — A field's `exit` reads how its value was committed, and its `change` the content types

Session 1956, a ledger round (K.2). Status: implemented. Extends ADR 2646 (`$event`) and ADR 2566
(focus events).

## Context

XFA 3.3 page 398's table has a field's `exit` set `commitKey` and its `change` set, among others,
`newContentType` and `prevContentType`. Page 402 gives `commitKey` four values: 0, not committed (the
escape key); 1, a click of the mouse outside the field; 2, the enter key; 3, a tab to a new field.
Page 403 defines the content types as `allowRichText` where the field supports rich text and
`plainTextOnly` where it does not, always the same in this version of XFA; page 865 makes a
`textEdit`'s `allowRichText` 0 where it is not stated and the `textEdit` holds no `exData`. All of
these were refused by name as properties a window does not tell this reader. Two of them it can tell
without any host saying more: `viewer-core` raises the `/Bl` of a tab itself (`Command::Focused`,
whose `Next` and `Previous` are what the tab key means) and of a press outside the widget itself,
and the content types are the template's.

## Decision

1. **`commitKey` is `pdf_model::view::CommitKey`**, the three ways of committing an interactive
   form field's script already reads with the same numbers (ADR 1626), rather than a second enum for
   the same concept. `Stimulus::Exit` carries `committed: Option<CommitKey>`, and
   `ViewState::activate_form_on_exit` takes it.
2. **The window says what it knows.** `Viewer::raise` takes how a `/Bl` among its triggers came
   about: a pointer's raise passes `Click`, since a pointer blurs a widget only by a press elsewhere;
   `move_focus` passes `Tab` for `Next` and `Previous`. A move to nothing (`FocusMove::None`, a
   host's gesture), a script's `setFocus` and a page turn pass nothing.
3. **A field's `exit` the window did not describe refuses `commitKey` by name** ("the window did not
   say how the value was committed"), which is a different sentence from the one for an event whose
   row of the table does not set it (a subform's `exit`, any `enter`). A choice list's selection
   commit is such an exit: `Command::CommitField` is a click on an item or Enter alike. Page 402's 0
   is no way a window this reader serves leaves a field, so it is not representable.
4. **The content types are the template's**: `Container::entry`, read from the `ui`'s `textEdit`,
   `allowRichText` where it states 1, or states nothing and holds an `exData` child — the page's
   literal condition, which the schema gives `textEdit` no child to meet, so a template that does
   not state the attribute allows no rich text. Every other `ui` is `plainTextOnly`.

## Consequences

`modifier`, `shift`, `keyDown` and `fullText` stay refused: each needs the keys held at a pointer's or
a focus's command, and those commands are built by every host (`viewer-ui`, `viewer-gtk`,
`viewer-qt`, `viewer-ffi`, the confined protocol), which is work, not a question.
