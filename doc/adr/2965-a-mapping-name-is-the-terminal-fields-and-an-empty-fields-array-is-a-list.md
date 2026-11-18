# 2965 — A mapping name is the terminal field's own, and an empty `/Fields` array is a list

Session 2073. Status: **implemented.** Context: review 2038 (findings B60, B61, B66), the owner's work order of
2026-11-11, ADRs 1062, 1198. Clauses: ISO 32000-2 §12.7.4.1 (Table 226), §12.7.6.2 (Tables 239, 240), §12.7.6.3
(Table 241).

## 1. `/TM` is not inherited, and an HTML form export carries it

Table 226 marks `/FT`, `/Ff`, `/V` and `/DV` inheritable and not `/TM`. `submission::mapping_name` walked the
`/Parent` chain, so a non-terminal field's mapping name renamed every terminal field under it. It now reads the
terminal field alone: the widget where §12.7.4.1's two dictionaries are merged (a `/T` on the widget), its
`/Parent` where it is a kid. Table 226's `/TM` is "\[t]he mapping name that shall be used when exporting
interactive form field data from the document", and the HTML Form format body (`urlencoded`, `multipart`) wrote
§12.7.4.2's name regardless; it now writes the mapping name where the field states one. Table 239's `/Fields` still
*selects* by §12.7.4.2's name. FDF and XFDF carry the field hierarchy by `/T` and are unchanged.

## 2. Absent and empty `/Fields` are two answers

Tables 239 and 241 make an absent `/Fields` decisive — every field, the Include/Exclude flag ignored. The reader
returned an empty vector for both an absent entry and `[]`, so `[]` with the flag clear (an include list naming
nothing) submitted and reset every field. `SubmitForm::fields` and `ResetForm::fields` are now
`Option<Vec<ResetTarget>>`: `None` is absent, and `Some(vec![])` selects by the flag as any list does — nothing
included, or nothing excluded. A `this.resetForm()` with no names is still every field. The type change reaches one
of the owner's audit tests (`inherited_reset_defaults_audit.rs`), whose literal for "no `/Fields`" becomes
`fields: None`; its assertions are unchanged.
