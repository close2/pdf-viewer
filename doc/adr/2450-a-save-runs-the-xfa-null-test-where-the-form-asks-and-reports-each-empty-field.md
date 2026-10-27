# 2450 — A save runs the XFA null test where the form asks, and reports each empty field

Session 1808, a clause round in batch 121. Status: implemented.

## Context

RFC 0011 section 5 step 4 is the `validate` element's tests without a script. ADR 2434 built the
format test. The null test was left, with the report a person sees. XFA 3.3 is cited by chapter and
printed page and paraphrased, never quoted (RFC 0011's preamble).

The brief's hypothesis was a check beside the format test. Half of it held: the test is a check of
the same element, run on the same value. The other half did not, because XFA 3.3 runs the two tests
at different moments and gives their failures different effects.

## Decision

1. **The levels are page 881's.** `nullTest` is `disabled` by default, and an empty field is then
   acceptable. At `error` the form object is invalid. At `warning` the person is told and may
   dismiss or override (page 384). A value the attribute does not list is read as the default, as
   `formatTest` is (ADR 2434). Page 381's table marks the test as applying to a field or an
   exclusion group, so it is run on those two kinds.
2. **When it is run is the form's choice, and saving is a moment it can name.** Page 396 runs the
   null test only at `execValidate()` or automatically during a pre-event the processor determines,
   so that a half-filled form is not flagged. Page 506 names saving to a file among those commits.
   Page 1340's `validate` option lists the activities validation runs before, `preSubmit` alone by
   default. This program reads `acrobat`'s option and falls back to `present`'s, as page 1340's note
   says the Acrobat family does, and runs the null test at a save only where the list holds
   `preSave`. All four hybrid forms in the corpora say `preSubmit`, so none of them is tested at a
   save. Running a test before a §12.7.6.2 submit is not built here.
3. **A failing null test is a report, not a refusal, and the empty value is written.** Page 1340
   says a save goes ahead whether validation succeeds or fails, and that the messages are still
   useful. Page 384 lets a processor refuse a commit while the form is invalid; it does not require
   it. Page 396 shows the field taking the cleared value, and the test failing only at the next
   pre-event. So the data takes the empty value, as the `/V` does, and §K.2's consistency holds.
   This differs from the format test, where page 162 holds the data back until the test succeeds
   (ADR 2434 §4).
4. **The test is of the value the save leaves in the field.** Page 381 says the test does not
   apply where the template gives a default value. Page 396's example reads that: a field holding
   its default is valid, the person clears it, and the test fails. So the default is why an
   untouched field passes, not an exemption for an emptied one. Null content means no value, an
   empty value, no choice selected, or an exclusion group with no button on. A check button that is
   off holds its off value, which is content.
5. **The report is the form's own words, grouped as the form says.** Page 383 has the processor
   show the designer's `message` for the test, or one of its own where there is none. Page 386's
   `validationMessaging` decides how messages are presented: each alone (the default), all
   together, the first only, or none. `Written::xfa_invalid` holds one list per message, with one
   sentence per field, and `viewer-core` sends each list as one `Event::Reported` with no page. That
   is the channel ADR 1592 uses for a `/K` refusal, shown by every window: quorra's `note:` line,
   the toolkit status bars, and the confined window's standard error. A format-test failure's
   sentence in `Written::xfa_unkept` now carries the designer's `formatTest` message too.

## Consequences

- The four hybrids state no `validate` and validate before submitting alone, so they save and read
  back as before (`crates/pdf-model/tests/xfa_datasets.rs`).
  `a_save_reports_each_field_its_null_test_holds_and_writes_it_empty` runs EXAMPLE 10.5's field and
  message at both levels and under both `validate` lists. Planting the `preSave` condition out
  makes that test fail (trap 13).
- Still owed in step 4: the script test (step 5's engine), the datatype test (page 386), and
  validation before a submit. Every field is tested by its PDF name, so a template field with no
  widget is not tested; an XFAF form gives each field one.
