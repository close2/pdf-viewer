# 3833 — A forbidden key is read through `states` everywhere, and a listed key skips null

Session 2283. Status: **accepted**. Context: review 2282 finding B01, ADR 3521; ISO 32000-2 §7.3.10,
§7.3.7; ISO 19005-2 sections 6.1.7.1, 6.1.12, 6.2.6, 6.3.3, 6.4.1, 6.4.2, 6.5.2, 6.9, 6.10, 6.11 and
their ISO 19005-4 twins (paraphrased, `doc/third-party-data.md`).

## The finding

`pdf_archive::table::states` is the validator's reading of two sentences of the base standard: a
reference to an object the file does not hold "shall be treated as a reference to the null object"
(§7.3.10), and an entry whose value is null is treated as absent (§7.3.7). Its comment said every
rule forbidding a key asks through it; twelve did not. They asked `Dictionary::get(..).is_some()`
or listed `keys(..)`, so `/AA 99 0 R` in a file with no object 99 was failed as an `AA`.

## What changed

1. Every site 2282 named asks `states`: a stream's `F`/`FFilter`/`FDecodeParms`, the `DocMDP`
   reference's digest keys, the catalog's and a page's `AA`, a widget's or field's `A` and `AA`,
   `XFA`, `NeedsRendering`, a configuration's `AS`, `AlternatePresentations`, `PresSteps`,
   `Requirements`, and an image's `Intent` or a graphics state's `RI`.
2. The two private `keys` helpers (`file_structure.rs`, `interaction.rs`) became one,
   `table::stated_keys`, which lists a dictionary's keys **skipping those whose value resolves to
   null**. Every rule that lists keys to report the ones its clause does not admit — the
   permissions dictionary, an appearance dictionary, an additional-actions dictionary, a signature
   field's appearance, the information dictionary — is a rule forbidding a key, so it reads the
   same way.
3. The converter's action removal (`pdf-transform`'s `archive::actions`) reads a holder's `/A` and
   `/AA` the same way, so a removal it lists is never of an entry that stated nothing. The other
   matching remedies (`external.rs`, the catalog and page removals) act on the report's places and
   already follow it.

## Cost

Two fixtures of this crate's own tests named an object their file did not hold for a key they
meant to state (`a_rollover_appearance_is_a_key_too_many`, the `AlternatePresentations` case of
`the_three_forbidden_keys_are_each_found_where_they_are_stated`); they now state a direct value.
`states` was measured at under two per cent of the predicates' time when it was introduced.
