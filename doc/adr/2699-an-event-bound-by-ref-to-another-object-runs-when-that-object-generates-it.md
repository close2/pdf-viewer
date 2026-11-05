# 2699 — An event bound by `ref` to another object runs when that object generates it

Session 1942, a ledger round. Status: implemented. Extends ADR 2626 (propagation) and ADR 2606, which
named every event whose `ref` named another object as not run.

## Context

XFA 3.3 page 706: an event's `ref` is "[a] SOM expression identifying the object which generates the
event". Page 707 says the ref points to the source of the event, not its destination. Page 404 calls
this the "come-from" notation. Before this round, only `$` (the event's own container), `$form` and
`$host` were followed.

## Decision

1. **Every event is bound to the objects its `ref` names.** `$` or no `ref` binds the event to its own
   container. Any other expression is read by `Step::path`, the SOM reader the JavaScript binding
   already used, which now lives in `pdf-model` so that one reader serves both. It is resolved from
   the event's container as a script's path is (pages 114–115) and binds to each field, subform and
   exclusion group it names. This is done once per activation (`Session::hooks`).
2. **An activity on an object runs every event bound to it, in document order of the holders.** Page
   404 leaves the order of one event's scripts open, and document order is the choice ADR 2506
   already made. Each event runs as its holder's script, so `$` is the holder and `$event.target` is
   the object (page 404's `target`).
3. **`listen="refAndDescendents"` applies to the object the `ref` names** (page 706: "Respond to the
   target and any objects it contains"). So the nearest-listener walk of ADR 2626 is keyed by the
   bound object, not by the holder.
4. **A `ref` that names nothing is named at the end of the open sequence, where ADR 2606 named it.**
   This covers no field, subform or exclusion group, `$layout` and the other DOMs this reader raises
   no event for, and an expression `Step::path` refuses.
5. **Binding is bounded twice (principle 3).** The objects bound in all are held to `Limits::names`,
   so a template of many events each naming `Row[*]` costs what the cascade's dependencies may cost.
   Each `ref` that searches the template is charged as many of `Limits::steps` as the template has
   containers, which is the most one level of the search visits. So a template of a hundred thousand
   containers that each name another ends on the steps limit and keeps nothing, as a cascade that
   does not settle does (page 380). It does not cost the square of its size. A qualified name such as
   `$layout` is refused before any search and costs nothing.

## Consequences

- A component dropped into a template with events that watch other fields, which is page 404's
  stated purpose for the notation, now works.
- Tests: `pdf-formcalc --test cascade` (`an_event_bound_by_ref_…`, three of them, and
  `events_bound_by_ref_are_charged_against_the_steps_limit`).
