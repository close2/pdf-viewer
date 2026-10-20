# 2131 — An incomplete schema description is cut from its container, and the complete ones stay

Session 1647, the archive slot of batch one hundred and one. Status: **accepted**, **built** in
`pdf-model`, `pdf-archive` and `pdf-transform`. It **narrows** the `discard` round 1639 built for
`metadata/extension-schema-container-fields`. Context: ADRs 1087, 2114; `doc/pdf-a-conversion-limits.md`
section 3.9; `doc/pdf-a-mitigations.md`'s entry for the row. Code: `pdf_model::xmp::remove_items`,
`pdf_archive::incomplete_schema_descriptions`, `crates/pdf-transform/src/archive/extension_container.rs`.

## 1. What the discard cost

ISO 19005-2 section 6.6.2.3.3's four tables say which fields a schema description states. A field the
packet states nowhere cannot be supplied without inventing metadata about metadata, so the
`discard` (section 3.9's loss, `metadata-property`) took the whole `pdfaExtension:schemas` property
and every property of every namespace it described, because the XMP writer could remove a property
and not one item of its array. A container holding a complete description beside the incomplete
one lost both, and both schemas' properties, though nothing failed about the first.

## 2. Decision

**Only the incomplete descriptions go, and only the properties no description left behind
describes.** `pdf_model::xmp::remove_items` takes the `rdf:li` items at given positions out of the
array one top-level property holds — found as `remove` finds a property, the array directly inside
it, the items counted in packet order — and every other byte stays the producer's.
`pdf_archive::incomplete_schema_descriptions` names the positions with the validator's own reading,
one description at a time: a description with any fault other than a misspelling. A namespace still
described by a kept description keeps its properties, since section 6.6.2.3.1 asks for a
description and one is there. What is left may still misspell a field; the respelling, which is
mechanical, answers it on the cut packet.

The whole container still goes where every description in it is incomplete, and where the packet
does not state exactly one container whose value is a bag — then the positions do not name items of
one array. The report names each description cut (`the schema description of <namespace>`) and each
property, and the history records the discard as before.

## 3. Evidence

`pdf_model::xmp`'s `an_item_goes_by_its_position_and_its_neighbours_and_their_fields_stay`;
`tests/archive.rs`'s `an_incomplete_description_goes_and_the_complete_one_beside_it_stays`, which
converts and conforms with the complete description and its property kept. The veraPDF census before
and after is `doc/history/1647`'s.
