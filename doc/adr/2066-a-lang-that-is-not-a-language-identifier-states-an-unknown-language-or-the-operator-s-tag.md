# 2066 — A `/Lang` that is not a language identifier states an unknown language, or the operator's tag

Session 1615, the archive slot of batch ninety-seven. Status: **accepted**, **built** in
`pdf-transform`. Code: `crates/pdf-transform/src/archive/languages.rs` (new),
`crates/pdf-transform/src/archive/{decision,prepare,rewrite,remedies,config,report,mod}.rs`,
`crates/pdf-transform/tests/archive.rs`, `doc/profiles/{as-shown,as-if-printed,only-metadata-loss,
keep-everything}.toml`. Clauses: ISO 19005-2 section 6.7.4; ISO 32000-2 §14.9.2.2, §14.9.2.3.
Section 2's content-stream half is ADR 2067.

## 1. What the standard states

ISO 19005-2 section 6.7.4 is a subclause of recommendations with one requirement in it: a `/Lang`
the catalog, a structure element or a marked-content property list does state shall be a language
identifier as the base standard's 14.9.2 defines one. Stating a language at all is a `should`, so
`pdf_archive` holds two rows to the one `shall` and none to the recommendations. ISO 32000-2
§14.9.2.2 is the definition:

> A language identifier shall either be the empty text string, to indicate that the language is
> unknown, or a Language-Tag as defined in BCP 47.

The brief read the subclause as wanting a language *for* the document and every element whose
language differs; that is the recommendation, not the requirement, and the rows were already right.

## 2. Decision

**The entry is kept and states one of the two values §14.9.2.2 admits.** Three routes were open:

- *correct the tag* — `en_GB` to `en-GB`, `EN-GB` followed by a NUL byte (sixteen of the real-world
  witnesses) to `EN-GB`. Each is this converter deciding what the producer meant, which
  `doc/questions/A48` forbids; the trailing NUL looks like a C string's terminator and is still a
  guess about a file this program did not write.
- *remove the entry* — §14.9.2.3: "If a structure element does not have a Lang entry, the element
  shall inherit its language from any parent element that has one", and a `Span` without one takes
  the enclosing language. A removal therefore *asserts* the parent's language over text whose
  producer said something else. That is worse than the loss it replaces.
- *write the empty identifier* — "to indicate that the language is unknown", which is exactly what
  a reader can know of a malformed tag. Chosen. **It is a loss** (`Loss::LanguageIdentifier`, word
  `language-identifier`): a lenient reader might have understood `en_GB`, and now falls back on its
  own default. The report's `restated_language_identifiers` names every entry with the value it held.

**`supply`** writes the operator's tag through the same two rewrites (`language = "de-DE"` per site);
the configuration reader holds the tag to the BCP 47 grammar the validator uses, so a supply cannot
write what the requirement refuses, and `language = ""` states no fact and stops. The catalogue's
`only-when = "malformed"` refinement is the construction itself — only an entry the site fails at is
written — so it is not a key. Two rewrites, one per site (`CatalogLanguageRestated`,
`MarkedLanguagesRestated`), so a profile may answer the catalog and the elements differently;
`only-metadata-loss` does, discarding the catalog's (the document's default, `dc:language`'s
counterpart) and stopping at a run of text's. `as-shown` and `as-if-printed` discard both (a language
is never drawn); `keep-everything` supplies with an empty tag, which stops until filled in.

A structure element written directly in its parent's `/K`, and a property list written directly in a
resource dictionary, are refused by name: the edit is made by object and those have none.

## 3. Evidence

The census's 63 witnesses at 2a (22 catalog, 41 element and property list, over the veraPDF corpus,
pdf.js, pdfbox and the format corpus): with `--authorise language-identifier` 32 convert and veraPDF
1.30.3 reports every one compliant (each input failing 6.7.4); the other 31 stop at another site
(the PDF 2.0 header, encryption, unembedded fonts, a forbidden annotation), none at these two. Every
page of the 32 is byte-identical to the input's at 72 and 720 dpi.
