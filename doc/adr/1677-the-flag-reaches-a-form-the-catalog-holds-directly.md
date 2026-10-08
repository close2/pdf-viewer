# 1677 — Table 224's flag reaches a form the catalog holds directly

Status: accepted and **built**. Session 1420. Closes the gap ADR 1665 section 3 found and did not
change.
Code: `crates/pdf-model/src/view.rs` (`Update::owe_appearances`, `Update::state_default_font`'s
catalog read; `interactive_form` removed).
Tests: `crates/pdf-model/tests/script_properties.rs`
(`a_style_with_no_on_state_is_owed_and_flagged_in_a_direct_form`,
`the_flag_and_the_free_text_font_are_written_into_one_form`).

## 1. The gap

A save that could not construct a widget's appearance sets Table 224's `/NeedAppearances`, whose
row binds a writer "if it has not provided appearance streams for all visible widget annotations
present in the document". The flag was written only where the catalog's `/AcroForm` was a
reference: a form held directly in the catalog got the report and not the flag. Table 29 types the
entry as a dictionary and states no indirectness, so the direct form is the document's form too.

## 2. The rule

- **A direct form is rewritten inside the catalog that holds it**, because an update replaces
  objects and a direct dictionary has no number of its own — the distinction
  `withdrawn_usage_rights` already draws for `/Perms`.
- **Both levels are read as the update has them** (`Update::current`). `state_default_font` may
  have written `/DR` into the form and `write_filings` may have rewritten the catalog; the flag was
  read from the file's own form and so overwrote a `/DR` font written in the same save. Plant that
  read back and `the_flag_and_the_free_text_font_are_written_into_one_form` fails.
  `state_default_font` now reads the catalog the same way, for the same reason.
- **A document with no form dictionary gets none for the flag**: a flag about every visible widget
  in a form whose `/Fields` would be empty describes a form the file does not have, and
  `Written::unconstructed` still names what is owed.
