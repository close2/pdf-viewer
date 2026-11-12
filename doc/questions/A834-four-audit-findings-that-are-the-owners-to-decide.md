Status: complete
Given: 2026-11-12, in conversation — transcribed by the round
Owes: (1) cut programmatic text values to /MaxLen in pdf-model and update the two audit tests in xfa_saved_values_audit and xfa_datasets to expect ten characters; (2) wire §12.2 /Enforce into the four-level restriction policy with default warn; (3) ensure redaction application always physically removes/destroys the underlying content in the redacted region first (never merely covering content with an overlay) and draw /RO after removal under §12.5.6.23's clause-driven relocation amendment; (4) keep reporting every unrecognised feature in Annex I.2 (audit / model / transform rounds)

> agree with all except 3.  If a redaction only places something over someting else, it will lead to sensitive information exposure.  We need to implement this correctly.

Reading: adopts recommendations 1, 2, and 4 as presented, and for 3 mandates that redaction is never a mere visual cover-up:
1. **`/MaxLen` in the model:** cut programmatic text values (`set_field`, scripts, imports) to `/MaxLen` in `pdf-model` and update `saving_xfa_text_fields_keeps_each_dataset_value_consistent_with_pdf_v` and `annotation_text_widget_keeps_its_text_values` to expect 10 characters.
2. **`§12.2 /Enforce`:** treat `/Enforce` as a document restriction under the four-level policy, with default `warn`.
3. **`Redact /RO` and true sanitization (§12.5.6.23):** redaction must always physically remove and destroy all underlying content (text codes, paths, and image samples, serialized as a new file via `crates/pdf-transform/src/redact.rs`) and must never merely place an overlay over unremoved content. Because `/RO` in Table 195 is drawn only *after* that physical removal has destroyed the underlying content in the redaction region, amend the overlay exclusion so "overlay" refers to `/OverlayText` (marks this program would compose) and place the producer's `/RO` Form XObject in the redacted region strictly after the underlying content has been removed.
4. **Annex I.2 error reporting:** keep reporting every unrecognised feature encountered rather than suppressing errors after the first.
