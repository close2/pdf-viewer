# 2566 — A hybrid form's `exit` and `enter` events run where a window moves the keyboard focus

Session 1866, a script round in batch 127. Status: implemented. Follows ADRs 2525 (the activation's
moments and its clock) and 2545 (the form's JavaScript realm).

## Context

§K.2's note owed "the events other than `change` and `initialize`". A census of the corpora's hybrid
forms, counted through `pdf-model`'s own template reader, chose which. In the two trees the tests read
(`doc/pdf.js` and `doc/corpora`) six hybrids read and three carry events, too few to rank. In the
crawl cache ADR 2482 counts, 151 static templates and 63 dynamic read: after `change` (2 177 events
in 43 static documents) come `exit` (1 491 in 45), `click` (941 in 69), `mouseUp` (363 in 13),
`ready` (297 in 42, almost all bound to `$form` by `ref`), `initialize` (258 in 27) and `enter` (193
in 21). Almost every one sits on a field. Every window already reports the focus: `/Bl` and `/Fo`
(Table 197) pass through `Viewer::raise` on a press, a tab, a script's `setFocus` and a page turn.

## Decision

1. **`exit` and `enter` are built as one pair, at one call site.** `Stimulus::Exit { left, reaching }`
   and `Stimulus::Enter { reached, leaving }` (`pdf_model::xfa`) each fire their activity on the
   containers the focus crosses, then settle the calculations that read what the events changed
   (page 379). `ViewState::activate_form_on_exit` and `activate_form_on_enter` raise them through
   the runner and the sequence clock that the merge and the entry use (ADR 2525 item 6). `viewer-core`
   calls them from `raise`: the `exit` at the `/Bl`, after the commit, so an `exit` script reads the
   person's entry (ADR 1592's order), and the `enter` at the `/Fo`, each before the widget's own
   scripts. No window, no `Event` and no wire message changes: an event script runs through
   `Runner::run` as a calculation does, in the realm of ADR 2545.
2. **What crosses (pages 390–391).** A field's events fire whenever it gains or loses the focus.
   A subform's and an exclusion group's fire only when the focus comes in from outside them or goes
   out of them, so the walk takes the field and its enclosing subforms and exclusion groups, less
   those that also hold the other field. A move to or from no field crosses every container around
   the field. The walk goes up `parent` links and is held to the template's container count; the
   XML reader's depth bound (256) makes it shorter still.
3. **Two orders the pages leave open, each a choice.** Within one half the pages do not order a
   container's event against its field's. `enter` runs outermost first, the order page 387 reports
   Adobe's processor dispatching a propagating event in, and `exit` runs innermost first, its mirror.
   Each half is its own activation, so a stopped `exit` does not prevent the `enter`.
4. **A stopped event changes nothing.** An event script that ends on a limit, or on the clock, ends
   its activation the way a calculation's does (ADR 2507). Every value set in that half is dropped,
   the report names the event and the limit, and the person's entry, committed before the `exit`,
   stands.

## Consequences

- Tests: `pdf-formcalc --test cascade` (the crossing in both directions, within one subform, out
  of the form, a value set by an event waking a calculation, a field the form does not name,
  a hundred nested levels, and one more `exit` event than an activation runs);
  `pdf-script-worker --features engine --test form_script` (an `enter` run in the confined worker
  that wakes a calculation, and the plant: three `exit` scripts that never end, each stopped
  within the budget with the entry kept); `viewer-core --test script_form` (a tab runs both at
  `run`, and runs nothing at `off`).
- **Owed under §K.2, in the census's order**: `click`, `ready` (bound by `ref` to `$form`, which
  the reader does not follow), `mouseUp` and `mouseDown`. Also owed: the `exit` and `enter` that
  page 392 fires on a choice list's new selection, and `listen="refAndDescendents"` (page 387).
  Pointer-driven `click`, `mouseUp` and `mouseDown` need one call site each, at Table 197's `/D`
  and `/U` in the same `raise`.
