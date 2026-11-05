# 2702 — A save and a submit fire their pre- and post-events, and a submit's `cancelAction` aborts it

Session 1944, a ledger round (K.2). Status: implemented. Extends ADR 2606 (`ready` and `docReady`)
and ADR 2646 (`$event`).

## Context

XFA 3.3 page 388 makes `preSave` and `postSave` application events, fired "just before the form
data is written out in PDF" and "just after the form has been written out". Page 389 has
`preSubmit` fire before the data is validated and submitted, with "the chance to examine and alter
the data before it is submitted", says it "applies only to the Form DOM ($form)", and fires
`postSubmit` after the returned data is marshalled and "[a]fter a cancelled or failed submit". Page
402: `cancelAction` "is cleared by the XFA processor at the beginning of an action such as
submitting data", setting it "causes the action to abort", and "[s]cripts must not attempt to clear
this flag". Page 398: a secondary event leaves the primary properties as the primary event set them,
and page 389's example 10.7 tells two submit buttons apart by `$event.target`.

## Decision

1. **Four stimuli** (`Stimulus::PreSave`, `PostSave`, `PreSubmit`, `PostSubmit`) fire the events of
   that activity bound to `$host` for a save and to `$form` for a submit, then the calculations that
   depend on what they changed, as `ready` does. An event of the activity bound elsewhere does not
   run: its `ref` names an object that generates no such event.
2. **The viewer raises them around the work.** `Viewer::save` runs `preSave` after the AcroForm's
   `/WS` and before `ViewState::save`, so a script's value is in the update, and `postSave` after a
   written save, beside `/DS`. A submit-form action runs `preSubmit` before
   `submission::compose`; `postSubmit` runs at once after a cancelled submit or a composition that
   failed, and for a submit the host transmitted, when `Command::Respond` brings its answer and
   before the answer is imported (page 389's order).
3. **The click is the primary event.** The widget of the click that raised the submit names its field
   (an exclusion group's field by ADR 2698's on value), and `$event.name` and `$event.target` are
   the click's. A save has no primary event and sets neither.
4. **`cancelAction` is carried** as a property node, as `change` is (ADR 2646): `Some(false)` from the
   beginning of an action, read as 0 or 1, set by any true value (page 1061's promotion; a
   JavaScript `true` arrives as 1), refused with page 402's sentence when a script writes a false
   value, and refused with page 398's when no action is under way. It survives an activation that
   ends on a limit: the flag was set before the action was consummated.
5. **A submit's `cancelAction` aborts the submit; a save's is reported and the file is written.** The
   submit is the document's own action toward a server the document names, so its script declining
   it restricts nothing of the reader's. A save is the reader's own operation, and page 1340 has a
   save written "regardless" of its validations; a document's script stopping it would be a
   restriction on the reader, which `CLAUDE.md` makes the reader's to set at four levels, and that
   policy is not built here. The save's `cancelAction` therefore stays on K.2's owed list.

## Consequences

- No wire change: `cancelAction` is a node the host answers for, in FormCalc and in JavaScript.
- A submit that a host transmits and that returns no answer fires no `postSubmit`: the host does not
  tell this crate that it ended. Named on K.2's owed list.
- Page 400's skipping of an action whose flag was set before its pre-event cannot arise: every action
  starts with the flag clear.
