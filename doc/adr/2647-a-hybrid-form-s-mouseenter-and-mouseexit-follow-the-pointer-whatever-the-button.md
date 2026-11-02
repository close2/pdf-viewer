# 2647 — A hybrid form's `mouseEnter` and `mouseExit` follow the pointer whatever the button

Session 1906, a script round in batch 131. Status: implemented. Follows ADRs 2584 (`click`) and
2627 (`mouseDown` and `mouseUp`).

## Context

XFA 3.3 page 391 lists `mouseEnter` and `mouseExit` among the field events: the pointer moving into
the field's region and out of it, whether a mouse button is up or down. Page 390 gives a subform
and an exclusion group neither. Table 197's `/E` and `/X` are the PDF counterpart, but §12.6.3's
first constraint lets an `/E` occur only with the button up, and `viewer-core` holds `Open::inside`
back while it is down; that constraint is the PDF clause's, not XFA's.

## Decision

1. **A second record of the pointer**, `Open::hovered`, follows the topmost annotation under the
   pointer on every pointer message whatever the button. When it changes, the old widget's field
   is given `mouseExit` and the new one's `mouseEnter`, in that order and before the `/X` and `/E`
   the same message may raise.
2. **Both are the field's own activities** (`Stimulus::MouseEnter`, `Stimulus::MouseExit`), each
   one activation under the runner, the level, the per-run deadline and the sequence clock, as a
   click is, with `$event` set (ADR 2646); a container hears them only by `listen` (ADR 2626).
3. **A form that states neither runs no activation at a crossing** (`Resource::has_activity`):
   the pointer crosses widgets on nearly every movement, and building the form's field table and
   activation for an event no template holds is work for nothing.

## Consequences

- Tests: `pdf-formcalc --test cascade`, `pdf-script-worker --features engine --test form_script`
  (both in the worker, and the plant: a `mouseEnter` that never ends is stopped within its budget
  and what it set is dropped), `viewer-core --test script_form` (a drag with the button down still
  enters and exits; at `off` nothing runs).
- Owed: the choice-list events of page 392.
