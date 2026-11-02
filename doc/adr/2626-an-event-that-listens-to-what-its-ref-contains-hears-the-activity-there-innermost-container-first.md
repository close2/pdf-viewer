# 2626 — An event that listens to what its `ref` contains hears the activity there, innermost container first

Session 1896, a script round in batch 130. Status: implemented. Follows ADRs 2525 (the activation's
moments and its clock), 2507 (a stopped activation keeps nothing), 2566 (`exit` and `enter`), 2584
(`click`) and 2606 (`ready`, and the report that named `listen` as not followed).

## Context

ADR 2606 read an event's `listen` and named every `refAndDescendents` event as run for its own
container alone. XFA 3.3 page 387 (Event Propagation) says that since XFA 3.0 an event object may
listen to events on its container and on everything the container holds, so events propagate
upward to enclosing containers, and that `refOnly`, the container alone, is the default for
backward compatibility; page 706 repeats both values and says the order of propagation is not
defined by the specification. Page 387 adds that the dispatch order is implementation-defined and
notes that Adobe's products dispatch propagating events in document order, outermost first, the
inverse of XML event processing. Neither page lists an activity that does not propagate. Page 388
applies `ready` to the Form DOM and the Layout DOM, and `docReady` is the application's.

## Decision

1. **An activity on an object reaches the events that listen above it.** Each time a `change`,
   `exit`, `enter`, `click`, `mouseDown`, `mouseUp` or `initialize` happens on a field, subform
   or exclusion group, the object's own events of that activity run, then the listening events of
   each enclosing container, innermost first, then the listening events bound to the Form DOM
   (`$form`), which contains every object of the form. Nothing else contains the object, so the
   walk goes no further. A listening event also responds to its own `ref`, as page 706 states, so
   a subform's listening `initialize` runs for the subform's own and once for each container in it.
2. **The object's own first and upward from it is a choice.** Page 387 leaves the order to the
   processor; its own sentence calls propagation upward, which is the order taken. Adobe's
   outermost-first order is the other one the page permits; ADR 2566's outermost-first `enter`
   orders different activities on different containers and is not changed by this.
3. **An activity on a DOM or the application reaches nothing above it.** `ready` happens on the
   Form DOM and `docReady` on the application, which no container contains, so a listening one runs
   once as one that does not, and a subform's own `ready` still names an object that generates none.
4. **The walk is bounded before the work.** For one activity a single pass over the template finds,
   for each container, the nearest listening container around it (the template numbers a
   container after its parent), so an object costs nothing where nobody listens and each step lands
   on a listener. Every event heard from below is charged one run of `Limits::activations`, even one
   that performs nothing, so an activity's dispatches are bounded by the runs an activation may
   make whatever a template nests or repeats; a stopped activation keeps nothing (ADR 2507).
5. **The report no longer names a listening event.** ADR 2606 item 4's sentence for
   `refAndDescendents` is withdrawn; its sentence for a `ref` naming another object stands.

## Consequences

- Tests: `pdf-formcalc --test cascade` (innermost first and the Form DOM last, `refOnly` and a
  sibling hearing nothing, each built activity heard after the field's own, a listening `ready` run
  once, two hundred and fifty levels each heard once, ten thousand and one heard events that perform
  nothing ending on the limit); `pdf-script-worker --features engine --test form_script` (the plant:
  a listening `click` that never ends, stopped within 5 s after the field's own event ran, and a
  listening `mouseUp` and `mouseDown` run in the confined worker).
- Memory high-water: ten thousand and one heard events 19 MiB against 18 MiB for as many of the
  field's own; the two-hundred-thousand-event template of ADR 2606 505 MiB, the figure ADR 2606
  records (debug builds).
- Owed: the `$event` object and its `target` (page 398), which a listening script reads to learn
  which object it heard; firing an event bound by `ref` to another object.
