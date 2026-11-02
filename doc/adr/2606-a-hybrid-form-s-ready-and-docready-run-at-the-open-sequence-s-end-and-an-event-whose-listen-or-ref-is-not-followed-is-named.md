# 2606 — A hybrid form's `ready` and `docReady` run at the open sequence's end, and an event whose `listen` or `ref` is not followed is named

Session 1886, a script round in batch 129. Status: implemented. Follows ADRs 2525 (the activation's
moments and its clock), 2507 (a stopped activation keeps nothing), 2566 (`exit` and `enter`, and the
census that ranks `ready`) and 2584 (`click`).

## Context

ADR 2566's census found 297 `ready` events in 42 static templates, almost all bound by `ref` to
`$form`. The template reader kept an event's `ref` only as "is it the container itself", so every
one of them was skipped in silence, and it read no `listen` at all, so an event on a subform whose
`listen` is `refAndDescendents` ran for the subform alone without a word (round 1875's finding).

XFA 3.3 page 388 defines `ready` as a DOM event, bound by a `ref` naming the DOM: for the Form DOM
(`$form`, alias of `xfa.form`) it fires once the template and the data are merged and the
calculations and validations have fired, which page 213 calls step 7 of the merge; for the Layout
DOM (`$layout`) once layout is complete and before rendering. Page 388 also gives the application
event `docReady`, before the document is rendered and after the Form DOM's `ready`; pages 387 and
707 bind application events through `$host`. Page 388 says the Form DOM's `ready` fires again when
the current data record advances, which a static form of one record never does. Page 387 defines
`listen`: `refAndDescendents` also hears the objects the `ref` contains, `refOnly` is the default
(page 706 repeats both). Page 706 makes the default `ref` the containing object, and page 703
requires the `ref` to name an object that can generate the activity, so a `ready` bound to a field
names nothing that fires.

## Decision

1. **`Stimulus::Ready` is the open sequence's end.** `ViewState::activate_form_on_merge` raises it
   after the merge's activation and the entries it raises again, under the clock the merge started,
   so a sequence raises it once. It fires the `ready` events bound to the Form DOM, then the
   calculations that depend on what they changed, then the `docReady` events bound to the
   application, then the calculations again — one activation under the reader's level, the per-run
   deadline and the one-second sequence clock (ADR 2525); a stopped one keeps nothing and leaves the
   form as the merge left it (ADR 2507). The merge already runs `initialize` after its validations
   (page 407's rule 3), so `ready` comes after both, as page 388 places it.
2. **The moment is ADR 2525's, not before the first frame.** ADR 2525 item 2 made the merge's
   completion the first frame (`Command::Presented`), so that an open pays nothing for a form it
   has not shown. `ready` and `docReady` inherit it: the first frame shows the AcroForm's own
   appearances, which in a conforming hybrid file are consistent with the resource (§K.2), and what
   the events set is drawn at the frame after. Page 388's "before rendering" is met for every value
   a script changes and not for that first frame; reversing it would put a script run on the launch
   path, which CLAUDE.md's *Nothing eager* forbids.
3. **`$layout`'s `ready` is not raised.** This reader lays out no XFA form — a static form's layout
   is the AcroForm's — so no moment here is page 388's "layout is complete"; such an event is named.
4. **The events not followed are named, by container and activity, in the same activation**: one
   whose `listen` is `refAndDescendents`, which runs for its own container alone and not for what it
   contains, and one whose `ref` names an object other than its container, the Form DOM and the
   application (page 404's inverted notation, `$layout` among them), which never runs. The walk is
   one pass over the template's events; the report keeps a sentence once and caps itself.
   Firing them is a later build.

## Consequences

- Tests: `pdf-formcalc --test cascade` (the Form DOM's events then the application's whatever the
  document order, a field-bound `ready` firing nothing, a merge alone running no `ready`, a value
  that wakes a calculation, `listen` named and a click within the subform still the field's own, one
  more event than an activation runs, two hundred thousand unfollowed events within the cap);
  `pdf-script-worker --features engine --test form_script` (a `ready` run in the confined worker,
  and the plant: three scripts that never end, stopped within 5 s, the merge's value kept);
  `viewer-core --test script_form` (`ready` runs at `run` and not at `off`, `$layout` named once).
- The memory high-water of the two-hundred-thousand-event template is the same under the open
  sequence's end as under the merge alone (about 505 MiB in a debug build, the template's own read).
- Owed: firing `listen`- and `ref`-bound events, `mouseUp` and `mouseDown`, page 392's choice-list
  events, a click within one member of an exclusion group, and performing `submit`.
