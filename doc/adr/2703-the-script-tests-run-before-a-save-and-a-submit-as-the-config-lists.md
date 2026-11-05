# 2703 — The script tests run before a save and a submit, as the `config` lists them

Session 1944, a ledger round (K.2). Status: implemented. Extends ADR 2696 (script tests in the
activation) and ADRs 2450, 2474 and 2475 (the null, datatype and format tests at a save and a
submit).

## Context

XFA 3.3 page 1340: the `validate` option "causes validations to be automatically run before
specified activities", `preSubmit` alone by default and an empty element preventing even that. Page
382 makes the script test one of a container's validations. ADR 2698 removed
`Activation::validations` on the ground that a save and a submit already run the validations, but
those ran without a script: the script tests are only reachable through the runner.

## Decision

1. **`PreSave` and `PreSubmit` test every container's script** where the `config` lists the activity,
   after the pre-event's scripts and the calculations they woke, so that what a `preSubmit` script
   altered is what is tested (page 389: `preSubmit` comes "before validation"). The whole form is
   tested, as a merge tests it, because the data a save or a submit writes is the whole form's.
2. **The failures are held for the host as an entry's are** (`Activation::script_tests`,
   `ViewState::take_form_validations`), grouped by `validationMessaging` and presented as page 383
   asks; a warning's dismissal returns the focus to the field (ADR 2698).
3. **Neither is cancelled by a failure.** The save is written regardless (page 1340). At a submit page
   1340 has an error cancel the action; the null, datatype and format failures are named and left to
   the host's submit policy (ADR 2475), and the script test is one of the same validations, so it is
   treated alike. Only a script's own `cancelAction` aborts a submit (ADR 2702).

## Consequences

- No post-event validates.
