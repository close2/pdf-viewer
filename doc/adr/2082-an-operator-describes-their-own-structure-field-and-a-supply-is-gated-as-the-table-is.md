# 2082 — An operator describes their own structure field, and a `supply` is gated as the table is

Session 1623, the archive slot of batch ninety-eight. Status: **accepted**, **built** in
`pdf-transform`. Context: ADRs 1245, 1990, 2050, 2066; `doc/pdf-a-mitigations.md`'s entry for
`metadata/extension-schema-structure-fields-are-described`. Code: `crates/pdf-transform/src/archive/
config.rs` (`Supplied::FieldDescriptions`, `field_descriptions_key`), `remedies.rs`
(`supply_field_descriptions`), `prepare.rs` (`prepare_field_descriptions`, `field_key`,
`undescribed_fields_in`), `mod.rs` (`supply_not_carried_out`).

## 1. What ADR 2050 left

ADR 2050 answers ISO 19005-2 section 6.6.2.3.1's structure-field half with a statement: the field's
namespace, name and serialised value type are the packet's, and Table 6's required
`pdfaField:description`, which no file states, is one fixed sentence saying the converter wrote it.
Its section 2 named the operator's own description as the configuration this version did not read.
A schema belongs to an organisation rather than to a document, so the operator whose pipeline writes
`acme:` fields is the one person who knows what each means, which is the catalogue's own argument
for `metadata/extension-schema-container-fields`' `supply`.

## 2. The configuration

`remedy = "supply"` with `field-descriptions = { "<namespace URI><local name>" = "<sentence>" }`.
**The key is the field's namespace and local name written together**, which is how RDF names a
property, so it does not depend on the prefix one packet spells the namespace with (section 6.6.2.2
makes a prefix mean nothing where none is required). Read when the configuration is: **an empty or
blank sentence is refused**, because Table 6 makes the description required and an empty one
describes nothing — written, it would meet the letter and tell a later reader less than the
converter's own sentence; a key naming no namespace is refused because it names no field. An empty
table is inert, for `role-map`'s reason.

## 3. What it does, and what it does not

The population is the fields the producer's own packets state and their value types leave out,
read as the requirement's row reads them (`undescribed_fields_in`, the same reading
`prepare_field_descriptions` makes, before any editor). Each field the table names is described
with the operator's sentence, named in the report's supplied facts and in `xmpMM:History`, and its
row in `described_extension_fields` says `described_by: operator`. **A field the table does not
name keeps the converter's sentence**: that is the site's answer with no configuration at all, so
naming one's own fields never stops a document over somebody else's. A table that names none of a
document's fields answered nothing, and the site keeps its `Stated` decision. Everything
`prepare_field_descriptions` refuses stays refused, whoever wrote the sentence.

## 4. A `supply` is gated by the preparation that carries it out

Until now a configured `supply` became `Decision::Configured` without asking the preparation of
the rewrite that carries it out; where that preparation refused, only stage 3's verdict on the
output stopped the file, and the report blamed the output rather than naming the reason. A supply
carried out by one of the table's own rewrites — this one, and the two language rows of ADR 2066 —
is now refused with the preparation's own sentence (`supply_not_carried_out`). Rewrites no
preparation stops have no obstacle, so the other supplies pass untouched. The test is the field in
another namespace than its type's under a table naming it: red before, refused by name after.

## 5. Evidence

veraPDF's `6-6-2-3-3-t03-fail-b` under a table naming `http://www.custom.com/1/field1`: converts to
2b, veraPDF 1.30.3 compliant (failing 6.6.2.3.1 before), the page byte-identical at 72 and 720 dpi,
the sentence in the packet and in the report. Four `pdf-transform` tests: the supply written, an
unnamed field keeping the converter's sentence, the empty and the namespace-less key refused when
read, and the gate.
