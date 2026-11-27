# 3803 — A rich text value's reading past its markup is logged

Session 2275. Status: **accepted and built** in `pdf-model` (`appearance::rich_text_value`).
Follows ADR 3795 and ADR 1197. Clauses: ISO 32000-2 §12.7.5.3 Table 231 bit 26, §12.7.4.3; XML 1.0
sections 4.1 and 4.6.

## Context

ADR 3795 made the rich text walk answer a reference no entity defines (`kept_reference`) and the
popup's `/RC` walk log it as `model/rich-text-malformed`. A text field whose bit 26 is set takes its
characters from the same walk (`rich_text_value`, ADR 1197) and logged nothing, so a field drawn
with `&nbsp;` kept as six characters reached no validator: trap 5's silence.

## Decision

Where the walk parsed an element — the value is markup — and either a token did not parse or a
reference was kept, the reading is logged as `model/rich-text-malformed` at the file
(`Site::WholeFile`), where the `/RC` walk places it, the value having no object of its own at this
point. A value with no element is plain characters the flag misdescribes, drawn as it stands under
ADR 0111's rule, and logs nothing: nothing in it was read past.

## Consequences

The class reaches section 5.1 for such a field, as it does for `/RC`. What the field draws is
unchanged.
