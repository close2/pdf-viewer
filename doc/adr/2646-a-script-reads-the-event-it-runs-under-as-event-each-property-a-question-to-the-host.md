# 2646 — A script reads the event it runs under as `$event`, each property a question to the host

Session 1906, a script round in batch 131. Status: implemented. Follows ADRs 2525 (the runner and
its clock), 2545 (the JavaScript realm) and 2626 (what an activity reaches above the object).

## Context

XFA 3.3 page 398 gives a script the `$event` object (also `xfa.event`) and tabulates which of its
properties each primary event sets, for each kind of object: `name` and `target` for every event of
a field and for a subform's `enter` and `exit`, nothing for an exclusion group's, and for a field's
`change` also `change`, `newText`, `prevText`, `selStart` and `selEnd`. A secondary event (a
calculation or validation the primary one woke) keeps the primary event's values, and using a
property the current event did not set is an error. Page 399 lets a script write `cancelAction`,
`selStart`, `selEnd` and `change` and no other property. Page 400 defines `newText` as `prevText`
with the span `selStart`..`selEnd` replaced by `change`, and that is what is placed into the field
once the event's scripts have run. Page 404 makes `target` the object whose change of state fired
the event, which is the field a listening container hears it from.

## Decision

1. **A property is a node.** `Form` holds the primary event (`Event`, `crates/pdf-model/src/xfa/
   calculate/event.rs`), and `Form::resolve` answers `$event.<property>` and `xfa.event.<property>`
   with a number past the template's containers, read by `Form::value` and written by `Form::set`
   as a container's value is; `$event.target` is the container itself, and a path continues from
   it. The numbers never enter what a calculation is recorded as naming, so `$event` wakes nothing.
   FormCalc reaches it through its accessors and JavaScript through its host's questions, so the
   wire between the host and the confined worker is unchanged.
2. **The primary event is set before each object's own events** in `Session::fire`, for the eight
   activities a person causes (`change`, `click`, `enter`, `exit`, `mouseDown`, `mouseUp`,
   `mouseEnter`, `mouseExit`), with `target` the object the activity happened on, and it stays for
   the listeners above it and the calculations the activity wakes. A merge, `initialize`, `ready`
   and `docReady` run under none, and a read there is refused as no person's action.
3. **Refusals are sentences, never nulls**: a property the event did not set (page 398), one the
   table sets that the window does not tell this reader (`modifier`, `shift`, `reenter`,
   `commitKey`, `keyDown`, `fullText`, the two content types), a name that is no property, and a
   write to a read-only property (page 399), `target` included.
4. **A commit is a `change` of the whole**: `Stimulus::Entry` carries what each field showed when
   the typing began (`ViewState::begin_typing` records it), so `prevText` is that text, `selStart`
   0, `selEnd` its length and `change` the entry; where the caller does not say, `prevText`,
   `selStart` and `selEnd` are refused as unsupplied. After a field's `change` events, a `newText`
   that differs from the entry is set into the field before the calculations run.

## Consequences

- Tests: `pdf-formcalc --test cascade` (a listening subform names the clicked field, a read-only
  write reported, an unset property refused, `change` rewritten, a span out of range refused);
  `pdf-script-worker --features engine --test form_script` (the same in JavaScript in the worker).
- Owed: `cancelAction` (no event this reader raises is cancellable yet) and the unsupplied
  properties above, which need the window to report modifier keys and keystrokes.
