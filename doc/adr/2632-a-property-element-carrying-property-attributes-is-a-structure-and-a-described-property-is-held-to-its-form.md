# 2632 — A property element carrying property attributes is a structure, and a described property is held to its form

Session 1899, slot 9 of batch 130, a "parsers" round (robustness).
Status: accepted and built in `crates/pdf-model/src/xmp.rs` (`Reader::attribute_form`, `Reader::open`,
`Reader::classify`), `crates/pdf-archive/src/table/metadata.rs` (`judge_against_description`,
`properties_outside_their_description`, the row `metadata/properties-use-known-schemas`) and
`crates/pdf-transform/src/archive/prepare.rs` (`prepare_properties`, `outside_their_schemas`).
Context: round 1881's "Found, not fixed" (ADR 2596); ADR 1991 (the 2005 edition is the predefined
schemas' source); ADR 1245 (the converter's described schemas). Q775 asks the owner about `T02-03_006`;
ADR 2633 is unused.

## 1. The gap was the reader's, and two instruments shared it

veraPDF fails `143424…`'s 2b output on ISO 19005-2 6.6.2.3.1 at object 553: `xmpMM:Ingredients`
"does not correspond to type Bag Text". The converter had written that description itself, from
`pdf_archive::undescribed_schemas`, because `pdf_model::xmp` read each item — an empty `rdf:li` carrying
`stRef:filePath` and nothing else — as the empty string. The W3C RDF/XML Syntax Specification (Revised,
2004), section 7.2.21, reads an empty property element with property attributes as a blank node whose
properties are those attributes (its section 2.5), and the XMP Specification of September 2005 admits
any valid RDF shorthand in a packet (p. 21). So the item is a structure, and the reader now says so: a
property, field or item element carrying an attribute that is not a namespace declaration, not RDF's own
and not `xml:`'s, and stating no `rdf:resource`, is structured, and its attributes are its fields. An
element with `rdf:resource` names its object rather than describing one and stays as it was.

With the reading right, `serialised_value_type` sees structures, the value type is undeterminable, and
the converter drops `Ingredients` under `undeterminable = "discard"` as it already dropped `Manifest`.

## 2. The checker judged a predefined property's form and not a described one's

The placement premise did not hold: a page-level packet inherits the catalog's schemas under
6.6.2.3.2, and the checker reads every metadata stream. What it lacked was any judgement of a
property an extension schema describes against the form its `pdfaProperty:valueType` names. The row
`metadata/properties-use-known-schemas` now asks it too. The form is read narrowly, so that a
reading this row does not understand under-reports rather than invents: `Lang Alt`; a simple type of
the edition's chapter 4 (pp. 73–86) as one string; a structured type of it, or a custom type a
description in reach defines, as a structure; a container's name before either, with its items so.
A type neither defines is `metadata/extension-property-value-types-are-defined`'s question; a
property its predefined schema defines is held to that schema only; ISO 19005's own namespaces are
their rows'. The description is looked for where `extension_schema_structure_fields` looks, in the
stream and in the catalog's packet. `prepare_properties` cuts what the new list names, exactly as
it cuts what the predefined half names, so the converter cannot keep a property this row fails.

The plant: the same packet with `Ingredients` described as `Bag ResourceRef` passes, in its own stream
and inherited from the catalog's (`a_property_is_held_to_the_form_its_description_gives_it`). HEAD's
2b output of `143424…` fails the row at object 553 and nothing else at 2b, as veraPDF fails it.

## 3. What the census showed

Both census runs against HEAD's, no-pack and with the CJK pack, moved no line: every summary line
(as-if-printed and as-shown, 2b and 4, both corpora) and every per-document line is HEAD's. That is
what the change predicts. The census lists refusals, `143424…` converted at HEAD as on the branch, and
what changed in its output (the `Ingredients` property dropped rather than described wrongly) is a
line of its report and a rule veraPDF checks, not a refusal. No attribute-form `xmpMM:History` item in
the corpora moved a line. veraPDF: the branch's 2b and 4 outputs of `143424…` pass, HEAD's 2b output
fails 6.6.2.3.1 alone.

## Consequences

An attribute-form `xmpMM:History` or `DerivedFrom` is now a structure where it was empty text, which
is what the predefined half of the row already expected of those properties' types.
