# 2475 — A submit runs the XFA validations and names what fails on the submission

Session 1820, a clause round in batch 122. Status: implemented.

## Context

XFA 3.3 page 1340's `validate` option lists the activities a form validates before, and `preSubmit`
is the default; an empty element prevents even that. Page 447's standard submit processing runs the
null test (step 4) and the format test (step 5) over the whole form whatever the submit carries,
and page 386 adds the datatype test. For every activity but saving, page 1340
has an error cancel the pending action and a warning let it go ahead. ADR 1592 is the channel a
refusal is said on; ADR 1198 is where a reader decides whether a submit goes.

The brief's hypothesis was that the submit calls `validate` with its event. It held. What the brief
did not say is where the sentences go: `submission::compose` returns a `Submission`, not a report
channel, and the same function already names Table 227 bit 2's Required field on
`Submission::owed` and leaves the decision to the host.

## Decision

1. **The same shape as Required.** `compose` runs the form's validations and names each failure on
   `Submission::owed`, one entry per message the person is shown, as page 386's
   `validationMessaging` groups them (`noMessages` shows none). It composes the request whatever
   is found. Whether an *error* cancels the submit is a policy a host supplies: refusing in
   `pdf-model` would be a refusal that cannot become an *ask*, which `CLAUDE.md` principle 3 names
   as the thing to avoid. The sentence says that page 1340 has an error cancel and that the host's
   submit policy decides.
2. **Whole form, current values.** The fields tested are every field of the form by the `/V` the
   view shows, not the ones the action selects (page 447, step 5's note).
3. **Which tests.** Null (ADR 2450), datatype (ADR 2474) and format. A save leaves the format test
   to the write, which refuses the value (ADR 2434); a submit has no write, so it runs it, with the
   level and the form's own message (page 383). The script test and the validation scripts of
   page 447's step 3 are a script's, and wait on RFC 0011 section 13's FormCalc engine.
4. **When it is asked.** `Config::validates_on_submit` is true where the `config` states no
   `validate` element, or lists `preSubmit`; false for an empty element or one listing other
   activities. `Config::default()` is a form without a `config` packet and validates before a
   submit.
5. **What is not validated is said.** A form that asks to be regenerated (Table 29's
   `/NeedsRendering`), a dynamic template (ADR 2380) and a resource that does not read each add one
   sentence saying the validations were not run, instead of passing.

## Consequences

A submit of a hybrid form whose fields hold values failing their types or carrying a required
field empty now lists them on `owed`, which hosts already print beside the request. A document with
no XFA resource pays one catalog lookup (`xfa::stated`). The four corpus hybrids state no
`nullTest`; whether their fields pass the datatype test is what a person's submit now says.
