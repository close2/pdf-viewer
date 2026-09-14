# ADR 1059 — The fallback names the page, and a pattern has none

## Status

Accepted, 2026-09-14. Session 1044. Takes the two readings ADR 1055 section 5 recorded as
questions for the clause, and answers both. Changes `pdf-model`'s `content.rs`,
`content/xobject.rs`, `content/text.rs`, `content/pattern.rs` and `pdf-archive`'s `survey.rs`; adds
`pdf-archive/tests/fixtures/resource-fallbacks/` with three documents, the tests over them in
`pdf-model/tests/missing_resources.rs` and `pdf-archive/tests/resource_fallbacks.rs`, and a
calibrated case in `pdf-archive/tests/cross_check.rs`. `§N` is ISO 32000-2 and nothing else.

## Context

ADR 1055's cross-check holds `crate::survey` to `pdf_model`'s interpreter at every resource
lookup, and over 2 908 corpus documents the two agreed 13 547 times. Its §5 says what such a gate
cannot decide: **a defect both machines share**. Two were recorded there, and both are about a
content stream that states no `/Resources`.

1. **A form XObject, and a Type 3 font.** Both machines fell back on the *invoking stream's*
   dictionary. §7.8.3 stated the fallback in a fourth bullet until Errata Collection 3 Issue #128
   retired it into an informative NOTE 3, and both the struck bullet and the NOTE name "the
   resource dictionary of the page on which they are used"; Table 93's `/Resources` cell says the
   same as a `shall` for the files that omit the entry, "all named resources used in the form
   XObject shall be included in the resource dictionary of each page object on which the form
   XObject appears". For a Type 3 glyph description the same erratum replaces §9.6.4's step d) and
   Table 110's fallback with a four-step search whose last two steps are the page and what
   §7.7.3.4 gave it. **Not one sentence anywhere names the invoker.** The two dictionaries are the
   same one until a form is nested inside a form with `/Resources` of its own, or a Type 3 font is
   shown from inside such a form — which is exactly why no corpus document could tell them apart.
2. **A tiling pattern.** Table 74 makes its `/Resources` "( Required )", and §7.8.3 requires it of
   "form XObjects, patterns, and annotation appearances" alike. The interpreter read a pattern
   stating none against nothing; the survey read it against the invoker's.

## Decision

**A stream that states no `/Resources` is read against the page's dictionary — with what
§7.7.3.4 inherited already in it — and a tiling pattern is read against nothing.**

The page's dictionary is held once per interpretation (`Interpreter::page_resources`, `Walk::
page_resources`) rather than threaded through every call, because it is one dictionary for the
whole of a page and every nested stream inherits the same one.

The pattern half is the decision a later round must not re-litigate: a required entry that is
absent is a malformed pattern, and the answer is **a refusal by name, not a fallback**. Every
name the cell uses is reported as a resource the file never defined, and what the cell draws
without a name is still drawn (trap 5). Inventing the invoker's dictionary here would put marks
on a page that no sentence of the standard puts there, and would do it silently.

Unchanged: the fallback is on the *entry's* absence and never on a name's (ADR 0255), and an
annotation appearance stream keeps the page's dictionary it already had — NOTE 3 names it too.

## Consequences

No corpus document witnesses either reading, which is trap 8's condition, so three fixtures carry
them: a form nested in a form, a Type 3 font shown inside a form, and a tiling pattern, each built
so that the page's dictionary and the invoker's disagree. They are cheap enough that
`cross_check.rs` runs over them **every round** rather than with the corpus, and that case is
calibrated by planting each old reading back into one machine only: the gate then names all three
by document, page, stream, operator ordinal, category and name. That is the proof ADR 1055
section 5 was owed — the cross-check is blind to a shared defect and sighted the moment the two
machines differ.
