# 1941 — A validation that is a script runs, and a calculated field's `warning` asks its choice

Session 1941, a single ledger round (K.2), the ledger lane. ADRs 2696 and 2697; no question.

**The slice.** Of K.2's owed list, the two a person sees: validations that are scripts (RFC 0011
section 5's step 4 script test) and page 642's choice to dismiss a `warning`. **The premise held**:
`template.rs` read no `scriptTest` and no `validate` script, `validate.rs` said the test waited on
an engine, the activation's `Runner` already runs both languages, and the warning notice had one
button and said so (ADR 2525). Effort was the only obstacle; nothing needed the owner.

**Script tests (ADR 2696).** `Resource::activate` runs every `validate` script whose `scriptTest`
is not `disabled` (page 881) once the calculations settle, before a merge's `initialize` events
(page 407). Scripts run on a copy of the form and their assignments are put back and reported
(pages 372, 385). A null or empty field is skipped (page 396), and true is page 1061's nonzero
promotion. A merge answers every container (page 382); any other stimulus answers those it changed,
those around them and those whose script read them. Failures go to `Activation::script_tests`,
grouped by `validationMessaging` (page 386), and `viewer-core` shows the designer's `scriptTest`
message or its own (page 383) with the entry kept.

**The warning's choice (ADR 2697).** The `warning` notice is a Yes/No question: "Yes" overrides;
"No", or a closed dialogue (ADR 1628), dismisses: `ViewState::answer_form_warning` raises
`Stimulus::Dismissed`, the field's calculation replaces the entry and its readers follow, with no
event fired.

**Tests**: `cascade` (7), `form_script` (1, JavaScript in the worker), `script_form` (2, 1 amended),
two unit tests. **Plant** (truth inverted, assignments kept, dismissal unseeded, empties tested): 8 of
69 fail.

**Left of K.2**: the events other than the eleven, a click within one field of an exclusion group,
an event's properties other than `script`, page 392's `exit`/`enter` on a choice list's selection,
an event bound by `ref`, the unset `$event` properties and `cancelAction`, page 385's choice for a
script test's `warning`, script tests at a save and a submit, and `validationState`. Effort only.

**Gates.** fmt `--check`, clippy `-D warnings --all-targets` (`pdf-model`, `pdf-formcalc`,
`viewer-core`, `pdf-script-worker`, and the last with `engine`): exit 0. nextest: `pdf-model` 2369,
`pdf-formcalc` + `viewer-core` 521, `pdf-script-worker --features engine` 56 passed, exit 0 each.
`cargo test -p conformance` after `git add -N`: exit 0. `state.sh docs`: exit 0. Tier 2: `pdf-model
script_corpus`, `selection_census`, `accessibility_census` exit 0; `corpus` (mixedfonts and issue15053
join, font faces), `raster_golden` (16 pages) and the engine `script_corpus` (threw 9315 to 4) exit
101, and exit 101 the same way with this round's `pdf-model` sources reverted to HEAD: the VM's.
