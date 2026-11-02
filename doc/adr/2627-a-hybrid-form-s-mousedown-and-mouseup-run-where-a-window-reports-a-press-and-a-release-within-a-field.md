# 2627 — A hybrid form's `mouseDown` and `mouseUp` run where a window reports a press and a release within a field

Session 1896, a script round in batch 130. Status: implemented. Follows ADRs 2584 (`click` at
Table 197's `/U`) and 2626 (what an activity reaches above the object).

## Context

XFA 3.3 pages 391–392 list `mouseDown` and `mouseUp` among the field events: the first fires when
the mouse button is pressed while the pointer is within the field's region, the second when it is
released while the pointer is within it, and page 392 warns that the two need not come in pairs,
since a press that slides out of the region before release fires the one without the other. Page
705 restates both. Page 390 gives a subform and an exclusion group neither. No page orders them
against `click`. Table 197's `/D` is a press inside the annotation's active area; its `/U` is a
release there, which §12.6.3's third constraint ties to a preceding `/E` and `/D` on the same
annotation, and `viewer-core` raises it only so.

## Decision

1. **`mouseDown` is raised at `/D` on a field's widget**, before the widget's own `/D` scripts, as
   ADR 2584 runs `click`: page 392's press within the region is `/D`'s condition.
2. **`mouseUp` is raised at every release over a field's widget**, wherever the press went down and
   whether or not the press selected text, and not only at `/U`: page 392 states the release alone,
   and `/U`'s §12.6.3 condition would drop a release a person dragged in from elsewhere. It runs
   before the `/U` the same release may raise, so a field's `mouseUp` precedes its `click`, the
   order in which the release comes before the click it completes; the pages state none.
3. **Both are the field's own activities** (`Stimulus::MouseDown`, `Stimulus::MouseUp`), each one
   activation under the runner, the level, the per-run deadline and the sequence clock, followed by
   the calculations that read what they changed. A container's events hear them only by `listen`
   (ADR 2626). A name that reaches an exclusion group is reported and not run, as a click is.

## Consequences

- Tests: `pdf-formcalc --test cascade` (the field's own and no subform's, an exclusion group
  reported); `pdf-script-worker --features engine --test form_script` (both in the confined
  worker); `viewer-core --test script_form` (a press on one field released on another runs the
  first's `mouseDown` and the second's `mouseUp`, and at `off` neither).
- Owed: `mouseEnter` and `mouseExit` (page 391), a press within one member of an exclusion group.
