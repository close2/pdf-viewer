# 2696 — An XFA form's validation scripts run in its activation, on a copy of the form

Session 1941, a ledger round. Status: implemented. Follows ADRs 2450, 2474 and 2475 (the null,
datatype and format tests) and 2506 (the activation).

## Context

XFA 3.3 page 382 lists four validation tests; the fourth, the script test, succeeds where the
`validate` element's script "must return a true value". Page 881 makes `scriptTest` `error` by
default, with `warning` and `disabled`; page 385 gives each level its response; page 383 says the
designer's `message` text named `scriptTest` is presented, or the processor's own words; page 386's
`validationMessaging` groups the messages. Page 382 runs every test at initialization and,
interactively, on exit from a field or subform a value was entered into, plus the tests the
processor infers by dependency tracking; page 396 says the script test is re-evaluated as content
changes and is not applied to a field holding null or empty content. Pages 372 and 385 say a
validation script must not alter the form and that a processor may be unable to enforce it.

## Decision

1. **The script tests run in `Resource::activate`**, once the calculations of any stimulus have
   settled and, on a merge, before the `initialize` events (page 407's rule 3), through the same
   `Runner` the calculations use, in both languages, under the same limits.
2. **Which are answered**: on a merge, every container (initialization). Otherwise those the
   stimulus changed, those around what it changed, and those whose script resolved what it changed
   while it ran — the dependency a calculation's own names give it (page 380). Every script is run
   to learn what it reads; a container no change reaches is not answered.
3. **A true value is page 1061's**: a value that converts fully to a nonzero number; null is false.
   JavaScript's `true` reaches the host as 1. This is FormCalc's promotion applied to both
   languages, a choice for JavaScript's text results, written down here.
4. **A validation alters nothing.** Every script runs on one copy of the form; what it assigns is
   put back before the next runs and is reported. Its text is charged to the activation's budget.
5. **A script refused or stopped decides nothing** and is reported, not failed. A limit that ends
   the activation drops the script tests with the calculations (ADR 2507).
6. **The host presents them** (`ViewState::take_form_validations`, `viewer-core`'s
   `validation_notice`): one alert per group, the designer's message or this program's words, the
   heavier level's icon, one button. The entry stands either way: page 383 recommends that a person
   not be held in a field until it validates.

## Consequences

- A `warning`'s dismiss-or-override choice (page 385) is not offered: neither choice changes a
  value, and the one thing override would change — no further warning at a commit — has no commit
  that runs script tests yet. A save and a submit run the tests without a script (ADRs 2474, 2475).
- `validationState` events (page 396) are not fired.
- Tests: `pdf-formcalc --test cascade` (the six `script_test` tests), `pdf-script-worker --test
  form_script` (JavaScript, in the worker), `viewer-core --test script_form` and its unit test.
