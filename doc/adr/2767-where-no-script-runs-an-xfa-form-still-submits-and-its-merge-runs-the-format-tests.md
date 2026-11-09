# 2767 — Where no script runs, an XFA form's events still submit and its merge runs the format tests

Session 1978, a ledger round (K.2). Status: implemented. Extends ADR 2738 (an event's `submit` is a
submit-form action) and ADR 2727 (the format test is no script); keeps ADR 1616's level unchanged.

## Context

XFA 3.3 page 703 gives an `event` exactly one of `script`, `submit`, `execute`, `signData` and
`encryptData`. A `submit` is therefore not a script: page 858 makes it the POST of the body its
`format` names, and ADR 2738 carries it out as the submit-form action it states, under the host's
submit level (`doc/questions/A98`). Page 382 says that "[w]hen an XFA-processing application
initializes a form, it executes all validation tests specified for the form", and its test 3, the
format test, is the `validate` picture's, not a script either.

At `off` — the default, and the confined window's pinned level (ADR 1616) — `ViewState` holds no
runner, and `run_form` returned before the form was activated: an entry was put to its format test
(ADR 2727) and nothing else happened. So a template's submit button sent nothing at `off`, although
the same button as an AcroForm `SubmitForm` action does, and the merge's format tests, page 382's
initialization, were not made there.

## Decision

1. **One activation, with or without scripts.** `Resource::activate_unscripted` runs the same
   activation as `activate_held` with `Session::scripted` false: the stimulus raises the events it
   raises where scripts run — its own, those bound to it by `ref`, the listeners above it, the
   `validationState` changes — and `dispatch` passes each `script` over, unrun and unsaid, while
   `perform` carries out every other property as before. `settle` (the calculations, page 379) and
   `script_tests` (page 385) return at once, since both are scripts. The format tests in
   `validated` run unchanged: at an entry over what changed, at the merge over every field.
2. **`ViewState::run_form` calls it where no runner is supplied**, for every stimulus, so a click,
   a press, a release, a crossing, a focus, an entry, the merge and `ready` each perform their
   events' `submit`s, and `send_form_submits` carries them out as it does at `on`: the form's
   `preSubmit` (its scripts passed over, the null, datatype and format tests run as page 1340
   asks), `compose`, `Event::Submit` to the host's submit level, `postSubmit`. The separate
   entry-only path (`format_test_unscripted`) is gone; its test is the activation's.
3. **Nothing is said of the scripts passed over.** The level's own sentence (Tier 0's naming of
   what it does not run, ADR 1616) says that the document's scripts do not run; a sentence per
   event at `off` would say it again at every pointer movement. A dynamic form at `off` is said
   nowhere but `notes::about`, as before.

## Consequences

- A hybrid form's submit button sends at `off` as an AcroForm one does, and the host's submit
  level, not the scripts level, decides whether anything leaves the machine: the two are separate
  questions in `CLAUDE.md` (a document's restriction versus a script's reach), and a `submit` is no
  script.
- A form presented at `off` whose data fails a format test puts the same notice it puts at `on`.
- A `validationState` event fires at `off` where the kept state changes; only what it holds that
  is not a script is carried out. ADR 2727's "no `validationState` event fires there" is read now
  as "no script of one runs".
- Tests: `crates/pdf-formcalc/tests/cascade.rs` (a click's submit held and its script not run, a
  `preSubmit`'s submit still refused, the merge's format tests) and
  `crates/viewer-core/tests/script_form.rs` (a click hands the host the submission at `off` without
  its `preSubmit` script, a `mouseUp`'s `urlencoded` submit at `off`, the merge's notice at both
  levels).
