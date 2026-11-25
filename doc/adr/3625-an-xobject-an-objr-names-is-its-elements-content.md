# 3625 — An XObject an /OBJR names is its element's content, on every route

Session 2231. Status: accepted. Context: ADRs 1100, 1970, 3285; review 2229 (B01, D01).

## Decision

A `Do` of a form or image `XObject` whose own `/StructParent` the structure tree claims (§14.7.5.3,
§14.7.5.4's Table 359, EXAMPLE 1's shape) records an `ObjectSpan` with `ObjectItem::XObject`: the
readback range it drew and the extent and refusal an `/MCID` sequence records, kept by the same
`OpenSequence` record (`open_object_item`). `by_absence` subtracts it, so its text is no artifact by
absence (§14.8.2.2.2); `Tree::logical_text` places it where the `OBJR` stands, once (§14.8.2.5.1),
annotations still staying out of it (ADR 1970); viewer-core attributes it to the element as a
`ContentItem::Object`, so the element is spoken with its text and placed by its marks. Bytes of the
object a sequence covers stay the sequence's (`structure::outside_sequences`, now public, §14.7.5.3
NOTE 1). Divided pieces carry the spans like `marked`, closed inside their `Do`.

## Cost

One `/StructParent` lookup per `Do` on a tagged page; a number-tree walk only where one is stated.
An annotation an `OBJR` names still contributes no text to its element in viewer-core: its place is
§12.5.2's `/Rect` and its control §12.7's, as before.
