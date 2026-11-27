# 3789 — A redaction empties the removed field's XFA values, or takes `/XFA` whole

Session 2272. Status: **accepted**. Context: ADR 3725, ADR 2396, ADR 2450, `doc/questions/A834`,
review 2271 finding B04; ISO 32000-2 §12.5.6.23, §K.2; XFA 3.3 chapter 2 (page 72), chapter 4,
page 77's hierarchy of DOMs, page 262.

## Context

`redact` removed a widget that met the region and, with it, its field's `/V`, `/RV` and `/DV`
(ADR 3725), and carried `/AcroForm`'s `/XFA` untouched, so the `datasets` packet still held the
value in a decoded stream any reader opens. §12.5.6.23 asks the processor to "remove all traces of
the specified content"; §K.2 binds a writer modifying a hybrid file: "The XFA field values shall be
consistent with the corresponding V entries of the PDF field objects." The owner's A834 makes the
removal physical.

## Decision

- **The fields** are those with a widget among the removed annotations, the set `traces::field_values`
  takes the value of (`pdf_model::view::widgets_by_field_name`, §12.7.4.2's qualified names).
- **Three places are the field's own**, and each is emptied (`pdf_model::xfa::redacted`): the
  `datasets` data value it binds, found by chapter 4's binding exactly as the save that keeps §K.2
  finds it (`Datasets::bound`, the template's `bind` and the default `once` by name); the `form`
  packet's field (the Form DOM written out, page 77; page 262 has an unbound field's value saved
  there), found by the PDF name read as a SOM expression (page 72) over that packet's containers;
  and the template's own `value`, the field's default — the counterpart of the `/DV` ADR 3725 drops.
- **A value is emptied, its node kept.** Binding is by name and by index among same-named siblings,
  so removing a data value would move every later sibling of its name onto another field; an empty
  data value is the null value §K.2's consistency asks of a field with no `/V`. A typed value's
  element (`text`, `integer`) stays and its content goes, so the content type survives. Every other
  byte is the producer's: the packet is spliced, as the save splices it (RFC 0011 section 5).
- **Where the places cannot be found with certainty, `/XFA` goes whole**, with `/NeedsRendering` if
  the catalog states it, and the report says why, as the converter's `XfaRemoved` does: a form that
  asks to be regenerated or whose template is dynamic, a resource that does not read, mapping rules
  not carried out, a template that names no such field, a binding not followed, a data value another
  field outside the region also binds (emptying it would break §K.2 for that field), and a `form`
  packet holding a field of that name its path does not reach. An uncertain splice is never written.
- An XFA stream the removal already restated is a refusal, never a stream carried as it was.

## Consequences

- `redaction_leaves_no_trace` carries the value in all three packets of an array-form resource and
  finds none after; a second fixture whose template names no such field loses `/XFA` and says so.
  2271's audit test (stream form) passes.
- A copy of the value in another field's data, or in a custom packet, is not the field's and is not
  touched — the same line ADR 3725 draws for text outside the region.
