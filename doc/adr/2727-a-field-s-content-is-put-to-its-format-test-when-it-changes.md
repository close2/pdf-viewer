# 2727 — A field's content is put to its format test when it changes

Session 1956, a ledger round (K.2). Status: implemented. Extends ADR 2696 (script tests) and ADR 2706
(validation state).

## Context

XFA 3.3 page 396: the script and format tests are re-evaluated in response to possible changes in
content, with dependency tracking deciding what else to re-validate, and neither is applied to a
field holding null or empty content; page 382: initialization executes every validation test, and
the format test passes where the `validate` picture can format the canonical value; pages 384–385:
`formatTest` is `warning`, `disabled` or `error`, and a failed test's message informs the person
that the value is recommended, or required, to conform to the input mask. This reader ran the format
test only before a submit and in a save's write.

## Decision

1. **An activation tests the fields whose content it changed, and a merge every field.** A format
   test reads only its own field's content, so the fields changed are the whole of its dependency
   tracking. A pre-event's own run stays the submit's (ADR 2475) and the write's.
2. **The failure is held as a script test's is**: `Invalid` carries its `test`, the level is
   `formatTest`'s and the message the `message` element's text named `formatTest` (page 383);
   failures go before the script tests', page 381's order, into the same `validationMessaging`
   grouping. Where the form gives no message, `viewer-core`'s words say what page 385 requires them
   to say: that the value is recommended, or required, to conform to the input mask. A `warning`
   asks dismiss or override as a script test's does (ADR 2698).
3. **The verdict is the validation state's** (`Test::Format` in `Validity`), so a change fires
   `validationState` (ADR 2706). A text this reader finds no canonical value for is the datatype
   test's where the datatype restricts it, as before a submit, and a picture this reader does not
   read is said and decides nothing.
4. **The test is no script, so it runs at every level.** Where no runner is supplied — the level
   `off`, which pins the confined window (ADR 1616) — a person's entry is still put to its field's
   test (`Resource::format_tests`), the failure held for the host and the verdict kept; no
   `validationState` event fires there, since an event is a script. The merge's run at that level is
   not made, since at `off` the merge activates nothing; page 382's initialization is owed there and
   K.2's note names it.

## Consequences

The test reads the canonical value, so a field with no edit picture whose person types the text its
picture would display (`ABC-123` for `AAA-999`) fails it: chapter 4's input parsing (pages 162–164)
takes that text as canonical, and it is not a value the picture formats.
