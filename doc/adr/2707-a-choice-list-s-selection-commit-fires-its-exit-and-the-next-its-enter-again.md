# 2707 — A choice list's selection commit fires its `exit`, and the next one its `enter` again

Session 1946, a ledger round (K.2). Status: implemented. Extends ADR 2566 (focus events) and ADR 2646
(`$event`).

## Context

XFA 3.3 page 392: from XFA 2.4, when a person commits a selection in a choice list by clicking it or
pressing Enter, an `exit` fires though the focus stays; a later selection while it keeps the focus
fires another `enter` and then an `exit`; each such `enter` is marked by `reenter` (page 403: false
for a field that is no choice list and for the first `enter` after the list gains the focus, true for
the later ones). Before 2.4 an `exit` fired at each selection and an `enter` only on the first, which
gave consecutive `exit`s, the behaviour 2.4 corrected. Page 392 requires 2.4's behaviour where the
template's schema is 2.4 or above, and page 393's `originalXFAVersion` processing instruction brings
back the earlier one where it names a version before 2.4 and states `LegacyEventModel:1`.

## Decision

1. **The window reports the commit.** `Command::CommitField` is what a host sends for a click on an
   item or Enter; where the focus this crate holds is on a widget of that field, the focus stays, and
   `viewer-core` raises `ViewState::activate_form_before_selection` before the commit and
   `activate_form_on_selection` after a commit the field took. The view state decides from the
   template whether the field is a choice list (`ui` holding `choiceList`).
2. **`Stimulus::Selected`** fires the list's own `exit` events; **`Stimulus::Reentered`** fires its own
   `enter` events with `$event.reenter` true, only after a `Selected` with no focus move since.
   Neither reaches the subforms around the list, which the focus did not leave.
3. **The focus then leaving fires no second `exit` of the list** (`Stimulus::Exit`'s
   `exited_at_selection`): the commit's `exit` already paired the last `enter`, and firing another
   would be the consecutive `exit`s page 392 says 2.4 corrected. This is a reading of the page, which
   does not state the case; the containers the focus goes out of fire theirs.
4. **The earlier behaviour** follows where the template's namespace names a version before 2.4, or its
   `originalXFAVersion` instruction does with `LegacyEventModel:1`: no re-entry, and the focus leaving
   fires the list's `exit` again. A namespace naming no version is read as the current schema's. The
   XML reader now keeps up to 16 processing instructions per element.
5. **`$event.reenter`** is false on every event of a field that is no choice list where page 398's
   table lists it (`click`, `enter`, `mouseDown`, `mouseUp`, `mouseEnter`, `mouseExit`), and on a
   choice list's `enter` it is as above; of a choice list's other events page 403 says nothing, so
   reading it there is refused by name.

## Consequences

The re-entry fires before the commit of the second selection, after the window has put the new
selection into the field, so an `enter` script reads the selection being made; the page does not
place the `enter` against the value. `preOpen` and `postOpen` (page 392) need a window that reports
a drop-down opening and stay owed.
