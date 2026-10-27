# 2474 — The XFA datatype test is the content clause of the field's value element, and runs with the null test

Session 1820, a clause round in batch 122. Status: implemented.

## Context

RFC 0011 section 5 step 4 is the `validate` element's tests without a script. ADR 2434 built the
format test and ADR 2450 the null test and the report. XFA 3.3 page 381's table lists a second test
of a field, the datatype test, and page 386 says what it is: the test is implied by a restrictive
datatype, an integer or a float for example, in the field's value object, its messages are
application-specific, and a failure makes the form object invalid. XFA 3.3 is cited
by page and paraphrased, never quoted (RFC 0011's preamble).

The brief's hypothesis was a check beside the null test. It held, with one addition: the datatype
test is of the *unformatted* value (page 382), so it needs what the format test already needed, the
canonical value a person's typing comes to under chapter 4's input parsing.

## Decision

1. **Which fields.** Only a field (page 381 marks a subform and an exclusion group not applicable),
   and only where its `value` holds `integer`, `decimal`, `float`, `boolean`, `date`, `time` or
   `dateTime`. A `text` value, `exData` and the image types restrict nothing the `/V` can say.
2. **What consistent means is each element's own content clause.** `integer` is an optional minus
   sign and decimal digits (page 755); `decimal` and `float` are an optional minus sign and decimal
   digits around a single period (pages 671 and 739); `boolean` is `0` or `1` (page 627); `date`,
   `time` and `dateTime` are chapter 24's canonical formats (pages 666, 868 and 668). A `decimal`
   also errs where it holds more fraction digits than `fracDigits` (default 2) or more leading
   digits than `leadDigits` (default no limit), the two sentences pages 671 and 672 print in those
   words. The template reader now keeps both attributes (`template::Digits`).
3. **Choices where the grammar is silent, documented as choices.** A point with digits on one side
   only (`.5`, `5.`) is read as the number it writes. A `date` or `time` element given a full
   date-time is no error, because pages 666 and 868 say the part it does not hold is truncated. An
   empty value is not tested: it has no content to be of a type, and page 396 leaves emptiness to
   the null test.
4. **The test is of the canonical value.** `datasets::canonical_text` is the first half of the code
   that writes a typed value into the data, lifted out so the save and the validation read a typed
   text the same way. A numeric field's text that is no number (page 156) fails the test as typed.
   A picture or a locale this reader cannot read, and a `dateTime` typed with no edit picture (whose
   default pages 165 and 1156 state differently), leave the test *unrun*, and the report says so
   rather than passing the field.
5. **Where it runs.** Wherever the null test runs (ADR 2450): a save whose `config` lists
   `preSave`, and a submit (ADR 2475). Its failure at a save is a report, not a refusal and not an
   exclusion from the data: page 1340 saves whether validation succeeds or not, and §K.2 keeps the
   data consistent with the `/V`.

## Consequences

`Written::xfa_invalid` now carries datatype sentences beside null ones, in page 381's order (null,
datatype) per field. The four corpus hybrids say `preSubmit` and no `preSave`, so no save of
theirs is changed. The error level is implied (page 386), so no sentence names a level.

Not built: a choice list's selected values and a check button's items are tested as the strings
they write (`validate::data_values`), without chapter 4's input parsing, which only an edit widget's
typing goes through.
