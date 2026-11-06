# 2732 — A field's events read the keys held, and its `change` the whole text typed

Session 1959, a ledger round (K.2). Status: implemented. Extends ADR 2646 (`$event`) and ADR 2726
(`commitKey`); reuses ADR 1762's key state and ADR 1771's crossing of it.

## Context

XFA 3.3 page 398's table has a field's `click`, `enter`, `mouseEnter`, `mouseExit`, `mouseDown`,
`mouseUp`, `exit` and `change` set `modifier` and `shift`, and its `change` (and `full`) set `keyDown`
and `fullText`. Page 403: `modifier` is "true if and only if the modifier key (the control or "Ctrl"
key on Microsoft Windows) was held down during the event", `shift` the same of the shift key. Page
402: `keyDown` is "true if the arrow key was used to make a selection, otherwise false", and
`fullText` is the full (untruncated) value where the field truncated what the person pasted. All four
were refused by name. ADR 2726 priced them as every host's pointer and focus commands gaining the
keys held. That premise did not hold: `Command::Keys` already carries `pdf_model::view::Keys`
(shift, modifier, arrows) from every host to the view state before the pointer or key that raises an
event, and across the confined protocol (ADRs 1762, 1771, 1786), for an interactive form field's
`event.shift`, `event.modifier` and `event.keyDown`. The one host that never sent it was the Windows
host, through `viewer-ffi`'s window ABI, which had no entry for it.

## Decision

1. **One key state, not a second.** `Resource::activate_held` takes `Option<Keys>`; the view state
   passes the keys it holds (`ViewState::set_keys`), and `activate_kept` passes `None`, which
   refuses `modifier` and `shift` by name ("the window did not say which keys it held"). A field's
   events in the table's rows set both. A subform's or exclusion group's events and a field's
   `validationState` do not. Page 399 lets no script write either.
2. **`fullText` and `keyDown` are recorded at the edit, not the commit.** This reader raises a
   field's `change` at the commit, but the arrow key and the text the window handed belong to the
   edit. `ViewState::set_field` holds, for a field being typed into, the whole text the window
   handed, after a keystroke script's rewrite and before `accepted` cuts it to the field's room,
   and `Keys::arrows` at that moment (`crate::xfa::Typing`). The commit's `Stimulus::Entry` hands
   it to the `change` event. `change` is what the field kept, `fullText` what was handed; they
   differ where the field truncated, which is page 402's case. A commit with no typing described
   (a merge's re-raised entry) refuses both by name.
3. **The Windows host tells its keys.** `viewer-ffi`'s `Window::keys` and `quorra_window_keys`
   tell the core what changed. `Window::press` tells it from the press's own modifiers, and the
   WinUI host sends Shift and Control before every pointer action.

## Consequences

`commitKey` stays unoffered where neither a tab nor a click took the focus. Enter on a choice list
arrives as `Command::CommitField`, which a click on an item sends too, and Escape cancels no
field's typing in any host (`viewer_host::keys` gives it to the selection and full screen). Page
402's 0 and 2 therefore need a gesture and a field on that command first. The confined window sends
no `Command::Keys`; its level is pinned `off` (ADR 1616), so no script there reads them. The C#
change is checked by `header_and_interop_agree` and is not compiled on this machine.
