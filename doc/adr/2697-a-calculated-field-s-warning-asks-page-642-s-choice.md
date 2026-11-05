# 2697 — A calculated field's `warning` asks page 642's choice, and "No" restores the calculation

Session 1941, a ledger round. Status: implemented. Amends ADR 2525, which put the `warning` as a
one-button notice that "does not pretend to ask" the choice.

## Context

XFA 3.3 page 642: where a calculation's `override` is `warning` and the person sets the value, the
processor presents a warning that the object is recommended to have a calculated value, with two
choices: dismiss, to leave the calculated value undisturbed, and override, to keep a value of the
person's choosing. The entry already stands under `warning` (ADR 2525); nothing could undo it.

## Decision

1. **One question, two answers.** `viewer-core` puts the notice as a Yes/No alert asking whether to
   keep the value entered. "Yes" is override and changes nothing. "No" is dismiss.
2. **A dialogue closed without a press answers "No"**, by `AlertButtons::dismissed`'s rule (ADR
   1628); "dismiss" is the page's own word for that answer.
3. **Dismissal is a stimulus of its own**, `Stimulus::Dismissed`: the field's calculation runs and
   replaces the entry, and the calculations that read it follow (page 379's cascade). No event
   fires, since nothing was entered. `ViewState::answer_form_warning` raises it, and the field is
   no longer one a person entered, which a later merge would otherwise leave alone.

## Consequences

- An `error` keeps its one-button notice: the calculated value has already replaced the entry.
- Page 642's "no warnings or prompts on subsequent gain of focus" holds already: the notice is put
  at an entry, never at a focus.
- Tests: `pdf-formcalc --test cascade` (`a_dismissed_warning_…`), `viewer-core --test script_form`
  (`a_warning_s_no_restores_…`) and its unit test.
