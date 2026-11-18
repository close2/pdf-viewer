# 2983 — An FDF file's `utf_8` and `utf_16` read an unmarked string in that form

Session 2076. Status: **implemented.** Context: review 2038 (findings B75, D07). Clauses: ISO 32000-2 §12.7.8.3.1
Table 246, §7.9.2.2.

Table 246's `/Encoding` is "[t]he encoding that shall be used for any FDF field value or option ( V or Opt in the
field dictionary ...) or field name that is a string and does not begin with the Unicode prefix ZERO WIDTH NO-BREAK
SPACE (U+FEFF)", with `utf_8` and `utf_16` among its allowed values. `forms_data::Encoding` read both as
§7.9.2.2's text-string rule, which for a string with no U+FEFF prefix is PDFDocEncoding: a UTF-8 name matched no
field, and a value or option saved as mojibake.

Now `Encoding::Utf8` reads such bytes as UTF-8 and `Encoding::Utf16` as UTF-16 without a byte order mark, which is
read big-endian. That last is a **choice**: the table names `utf_16` and states no byte order; §7.9.2.2.1 gives every
UTF-16 text string big-endian order, and a string carrying no mark can state no other. Bytes that are not the named
form are refused by name on the field (`/V: bytes that are not the file's /Encoding`), never read as another
encoding. Because the value and options travel on as the file's own objects, which everything downstream reads as
text strings, a decoded `/V` (or each string of an array `/V`) is carried on re-encoded as a text string. XFDF's
reader, whose values are already text strings, says so with `Encoding::TextStrings`.

D07: Table 246's second exclusion, `/Status` beside `/Pages`, is now named as the first is, so the comment that said
both were named is true.

Tests: `pdf-model` `fdf_utf8_import_spec_audit` (three), `forms_data::tests::utf_16_and_utf_8_read_unmarked_strings_in_their_own_forms`,
`forms_data::tests::status_beside_pages_is_named`.
