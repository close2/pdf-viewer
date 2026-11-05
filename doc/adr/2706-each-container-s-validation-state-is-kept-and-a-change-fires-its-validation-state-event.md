# 2706 — Each container's validation state is kept, and a change fires its `validationState` event

Session 1946, a ledger round (K.2). Status: implemented. Extends ADR 2696 (script tests in the
activation), ADR 2703 (the script tests before a save and a submit) and ADRs 2450, 2474 and 2475
(the null, datatype and format tests).

## Context

XFA 3.3 page 396 fires `validationState` for each field, exclusion group and subform when the object
is first initialized, and afterwards only when it changes from valid to invalid or back, or when the
reason for invalidity changes (its example: from the null test to the script test). Page 397 makes a
container invalid where any of its content fails a validation and valid where none currently does,
no test evaluated included. Page 396 also says when each test runs: the null test only at an
explicit `execValidate()` or in a pre-event, the script and format tests whenever content may have
changed. `Resource::activate` was stateless — the form is rebuilt from the field values each time —
so nothing could know a state had changed.

## Decision

1. **`pdf_model::xfa::Validity` is the kept state**: per container, the set of tests (`Test::Null`,
   `Datatype`, `Format`, `Script`) its last run of each failed, and the state each object's event
   last announced. A container's state is the union of its own set and of every container inside it
   (page 397). `Resource::activate_kept` takes it by `&mut`; `Resource::activate` keeps its signature
   and passes a fresh one, so a merge announces every state and no other stimulus announces any.
2. **A verdict stands until its test runs again.** The script tests an activation answers record
   their verdicts (a field holding nothing is put to none, page 396, so it fails none); `PreSave` and
   `PreSubmit`, where the `config` lists the activity, record the null and datatype tests and, at a
   submit, the format test (`validate::verdicts`, the same tests `validate::before` names). A field a
   person clears therefore stays valid until the next pre-event's null test, page 396's own case.
3. **When it fires.** At a merge, after the `initialize` events, every field, exclusion group and
   subform in document order — page 396's "first initialized", and the order and the place after
   `initialize` are choices; afterwards, at the end of any activation, each object whose state
   differs from the one announced, then the calculations they woke. The validations are not run
   again for what a `validationState` script changed, which would let two scripts wake each other.
4. **`$event`**: page 398's table has the event set `name` and `target` on all three kinds of object,
   an exclusion group included.
5. **An activation that ends on a limit restores the state it was given**, as it drops its edits.
6. **The window holds the state** (`ViewState`'s scripting state), from one activation to the next.

## Consequences

The format test still runs only at a submit and in the write, not at an entry, so page 396's
re-evaluation of the format test when content changes is not followed; a field whose format test a
submit failed stays invalid until the next submit. That gap predates this decision and is named in
K.2's note.
