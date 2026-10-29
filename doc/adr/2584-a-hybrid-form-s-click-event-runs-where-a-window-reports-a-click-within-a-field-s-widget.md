# 2584 — A hybrid form's `click` event runs where a window reports a click within a field's widget

Session 1875, a script round in batch 128. Status: implemented. Follows ADRs 2525 (the activation's
moments and its clock), 2507 (a stopped activation keeps nothing) and 2566 (`exit` and `enter`).

## Context

ADR 2566's census of the crawl cache ranked `click` first among the events left (941 events in 69
static templates). XFA 3.3 page 391 lists `click` among the field events: it fires when a mouse
click occurs within the field's region. Page 390 lists the subform's and the exclusion group's
events, and `click` is not one of them. Page 387 lets an event element hear its container's
descendants only where its `listen` property is `refAndDescendents`; the default is `refOnly`.
Page 703 gives an `event` five one-of properties, `script` among them, and the template reader kept
an event only where it held a script, so an event that submits passed as an event that does nothing.

## Decision

1. **The trigger is Table 197's `/U` on a widget.** A click is a press and a release both within the
   region, and `viewer-core` raises `/U` only for a release inside the annotation a `/D` went down on
   (§12.6.3's third constraint). `raise` calls `ViewState::activate_form_on_click` there, before the
   widget's own `/U` scripts, as the focus events run before the widget's own. The press has already
   moved the focus, so a commit and the `exit`/`enter` pair come first. `mouseDown` and `mouseUp`
   are `/D` and `/U`'s own events and are not this decision.
2. **A click fires the field's own `click` events and no container's** (`Stimulus::Click`). Nothing
   bubbles, because page 390 gives the containers no `click` and `listen` is not followed (owed, with
   `ref`, since ADR 2566). After the events, the calculations that read what they changed run (page
   379). It is one activation under the runner, the level, the per-run deadline and the sequence
   clock that the other stimuli use, and a stopped one keeps nothing and leaves every entry as the
   person made it (ADR 2507).
3. **Two cases are reported and not run.** An exclusion group's AcroForm field is one for all its
   buttons, so its name does not say which member field the click was within; the report says
   so. A name that reaches a container that is not a field is reported the same way.
4. **An event's other one-of properties are kept and reported** (`template::Performs`). `submit`,
   `execute`, `signData` and `encryptData` are read and said where their event fires: "its click
   event's submit is not performed". This holds for every activity, so an `exit` or an
   `initialize` that submits is now reported too.
5. **Keyboard activation raises no `click`.** Page 391 names only a mouse click. Page 479 has a button's
   click activated when the person selects it without saying how, and no window here raises `/U`
   from a key, so the viewer has no keyboard trigger
   to attach a click to.

## Consequences

- Tests: `pdf-formcalc --test cascade` (the field's own events and no container's, a value that
  wakes a calculation, `submit` reported, an exclusion group reported, an unnamed field, one more
  event than an activation runs, a hundred thousand submitting events reported once);
  `pdf-script-worker --features engine --test form_script` (a click run in the confined worker, and
  the plant: three scripts that never end, stopped within 5 s, the entry kept);
  `viewer-core --test script_form` (a click runs at `run` and not at `off`, and neither a press on
  no widget nor a press and release on two different places is a click).
- Owed: a click within one member of an exclusion group, which needs a widget mapped to its member
  field; performing `submit` and the other properties; `ready`, `mouseUp`, `mouseDown`, page 392's
  choice-list events, and `ref`- or `listen`-bound events.
