# 2756 — Escape abandons a field's typing, and its `exit` reads `commitKey` 0

Session 1972, a ledger round (K.2). Status: implemented. Extends ADR 2755 and ADR 1592 (a field's
commit); revises the field-typing Escape of `quorra` (ADR 1216) and `quorra-confined`.

## Context

XFA 3.3 page 402 gives `$event.commitKey` 0 as "The value was not committed (for example, the
escape key was pressed)", one of the four things that happen to a field's value "when the form
field lost focus". No host had a gesture that abandons typing: `quorra` and `quorra-confined` gave
Escape in a field the meaning "the keyboard back to the page", the typed value kept and committed
later by whatever moved the focus; `viewer-gtk`, `viewer-qt` and the WinUI host gave it nothing in
a field. `viewer_host::keys` binds Escape on the page to the selection, full screen and a warned
draw, and already states that a field holding the keyboard takes a key before the page does.

## Decision

1. **Escape in a field that has the keyboard abandons the typing.** The order is the one
   `viewer_host::keys` writes down ("Chrome takes a key before the page does"): the field has the
   key first, and the page's three meanings are untouched. The hosts' convention for Escape is
   *not that* (the same module's argument for the warned draw), and page 402 names the key as its
   example of a value not committed. Every host takes it: `quorra` and `quorra-confined` in their
   typing keys, `viewer-gtk` by a key controller on each text control, `viewer-qt` in the event
   filter that already gives a note's editor Escape before the window's shortcut, the WinUI host in
   the text controls' `KeyDown`, `viewer-ffi` by `quorra_cancel_field` and
   `quorra_window_cancel_field`. Each then gives the keyboard back to the page, as Enter does.
2. **`Command::CancelField` puts the field back and takes the focus off it.**
   `ViewState::abandon_typing` restores what each widget showed before the typing began (the record
   a refused commit already restores) and runs the one-call calculations again, with no commit;
   `Done::AbandonField` logs it, so a replay does not type the characters again. Where the focus
   `viewer-core` holds is on the field's widget, it leaves for nothing: Table 197's `/Bl` is raised,
   its commit finds nothing typed, and a hybrid form's `exit` events read 0
   (`Exited::NotCommitted`). A toolkit's own control holds a focus `viewer-core` does not see, and
   its loss is the toolkit's.
3. **A file-select control in `quorra` chooses its file at Enter** rather than at Escape (§12.7.5.3,
   ADR 1216's reason for choosing once): Escape now means the pathname is abandoned.

## Consequences

A multi-line field in `quorra`, where Enter is a character, is left with its typing kept by a press
elsewhere or a tab, and abandoned by Escape. The AcroForm side has no `commitKey` 0 to set:
ADR 1626 carries it as "no commit", and an abandoned typing runs no commit-time script.
