# 1944 — A save and a submit fire their pre- and post-events, and a submit's `cancelAction` aborts it

Session 1944, a single ledger round (K.2), the ledger lane. ADRs 2702 and 2703; no question.

**The slice.** Of K.2's owed list, what happens when a person saves or submits: `preSave`,
`postSave`, `preSubmit` and `postSubmit` (pages 388–389), `$event.cancelAction` (pages 387, 399–402)
and the script tests at a save and a submit (page 1340). One coherent action model, and a person
sees each part. Page 392's choice-list events still need the window to report a selection commit,
and `validationState` needs a validation state kept across activations; both are left whole.
**The premise held.** No stimulus raised the four events, `cancelAction` was in `ELSEWHERE` and
refused, and `before_save`/`before_submit` ran only the tests without a script. Effort only.

**Events (ADR 2702).** `Stimulus::PreSave`/`PostSave` fire the activity's events bound to `$host`;
`PreSubmit`/`PostSubmit` those bound to `$form` (page 389: "applies only to the Form DOM"). The
viewer runs `preSave` after `/WS` and before the update is written, `postSave` after a written save,
`preSubmit` before `submission::compose`, and `postSubmit` at once after a cancelled or failed
submit, else when `Command::Respond` brings the answer, before its import. The click that raised a
submit gives `$event.name` and `target` (page 398; example 10.7).
**`cancelAction`.** A property node cleared at an action's start, read 0/1, set by a true value,
refused when cleared (page 402) or outside an action (page 398). A `preSubmit` that sets it aborts
the submit, said by name; a `preSave` that sets it is reported and the file is written, since a
save is the reader's (CLAUDE.md principle 3).
**Script tests (ADR 2703).** `PreSave`/`PreSubmit` test every container where the `config` lists
the activity, after the pre-event; failures are held for the host and cancel nothing (ADR 2475).

**Tests**: `cascade` 5 new, `script_form` 1, `form_script` 1 (JavaScript in the worker). **Plant**
(flag never set, `preSubmit` on `$host`, no tests at a save, no primary event, `preSave` after the
write): 5 of 111 fail.

**Left of K.2**: the other events, an event's other properties, page 392's choice-list events, the
unset `$event` properties, a save's `cancelAction` (the reader's four levels), a transmitted
submit's `postSubmit` with no answer, and `validationState`. Effort only. `notes::about` is stale.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` (`pdf-model`, `pdf-formcalc`,
`viewer-core`, `pdf-script-worker`, the last also with `engine`): exit 0, after two functions past
clippy's hundred lines were split. nextest of the four with `engine`: 2962 run, 2919 passed, the 43
others the decoder tests that need `pdf-sandbox-worker` beside them (trap 10), which pass, 88 of 88,
once it is built. `cargo test -p conformance` after `git add -N`: exit 0. `state.sh docs`: exit 0.
Tier 2: `pdf-model` `script_corpus`, `selection_census` and `accessibility_census` exit 0; the core's
`corpus` and `raster_golden` and the engine `script_corpus` not run, since no runner reaches a page.
