# Q834 — Four audit findings that are the owner's to decide

Source: round 2000, the audit-fix round (`doc/reviews/1983-specification-audit/fixes.md`). Every other
finding of the audit is fixed, or argued from its clause as not a defect, in that file. These four need a
word only you can give: a scope sentence of `CLAUDE.md`, a default of a restriction policy, a departure
from a `should`, and two of the audit's own tests that contradict a third reading. Each has a
recommendation; nothing waits on the answer.

## 1. Does the model cut a programmatic text value to `/MaxLen`?

Table 232: `/MaxLen` is "The maximum length of the field's text, in characters." Round 2000 made every
control a person types into hold to it (ADR 2816 section 1). Cutting a value set through the model as well
(a script, an import, `set_field`) was built and withdrawn, because two passing tests of the audit expect
the opposite: `pdf-model::xfa_saved_values_audit
saving_xfa_text_fields_keeps_each_dataset_value_consistent_with_pdf_v` and `pdf-model::xfa_datasets
annotation_text_widget_keeps_its_text_values` set fourteen characters into `annotation-text-widget.pdf`'s
`Tekstveld2`, which states `/MaxLen 10`, and expect all fourteen saved.

**Recommendation:** cut in the model too — the cell states the field's text, not the keyboard's — and
revise those two expectations to ten characters.

## 2. What level does §12.2's `/Enforce` take?

`/Enforce` (Table 147) names viewer preferences "that shall be enforced by PDF processors and that shall not
be overridden by subsequent selections in the application user interface" — a document restricting what its
reader may choose. It is reported and not obeyed (ADR 2814); `/HideMenubar`'s departure is kept. Under
*A document's restrictions are the reader's to set* it belongs to the four-level policy, whose default the
principle does not state.

**Recommendation:** treat it as a document restriction under that policy, default `warn`.

## 3. Is a `Redact` annotation's `/RO` inside the overlay exclusion?

`CLAUDE.md` keeps "the redaction's overlay and its fill" on the far side of the authoring line. The audit
(prose item 15) found ADR 1124's reason false: `/RO` is a Form XObject the producer wrote, not content the
file did not hold, and Table 195's drawing of it after application is a `should`.

**Recommendation:** read "overlay" as `/OverlayText` (text this program would set) and treat `/RO` as a
producer's marks placed where §12.5.6.23 says, under the clause-driven relocation amendment; then build
drawing `/RO` after application.

## 4. Does Annex I's first error suppress the rest?

Annex I.2: "Upon the first error that is caused by encountering an unrecognised feature, the PDF processor
should notify the user that an error has occurred but that no further errors will be reported." This program reports every unrecognised feature it meets (trap 5: an
error not reported is drawn as though it were the producer's intent). ADR 2815 section 2 records that as a
departure from the `should`'s second half.

**Recommendation:** keep reporting every one; the departure is louder than the standard asks, never
quieter.
