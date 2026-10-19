# 1991 — The XMP Specification part 2 names is the September 2005 edition

Session 1577, the archive slot of batch ninety-two. Status: **accepted**, **built**. Context:
`doc/history/1571` (veraPDF 1.30.3 held `102952.pdf`'s 2b output to ISO 19005-2 section 6.6.2.3.1
on four properties this validator passed); `crates/pdf-archive/src/table/metadata.rs`'s
`PREDEFINED`, which already *was* that edition's tables.

## 1. Which edition part 2 names

ISO 19005-2 section 6.6.2.3.1 admits a property of a predefined schema of "the XMP
Specification", ISO 19005-1 or part 2 itself, or of a described extension schema. **Clause 2, the
normative references, does not list the XMP Specification at all** — checked in
`doc/pdfa/ISO_19005-2_2011.md` and in the SRPS 2020 text. So clause 2's rule that an undated
reference takes the latest edition, which binds the documents clause 2 lists, does not reach it.
The one place the part identifies a document by that name is Bibliography entry 20: *XMP:
Extensible Metadata Platform*, September 2005. That edition is the one read.

The tree's earlier reading — "cited without a date, so a later edition's additions are predefined
too" — applied clause 2's rule to a document clause 2 does not name. It also made `xmp:Rating` a
real (Part 2 of the current specification) where the 2005 edition gives a closed choice of
integers; that row returns to the 2005 type.

## 2. The four properties, read in the texts

Fetched as evidence and kept in the round's scratch only (the 2005 edition through the
bibliography's own address, archived; Parts 1 of 2012 and 2 of 2022 from Adobe's XMP toolkit):

| property | 2005 | Part 1 (2012) | Part 2 (2022) |
|---|---|---|---|
| `xmpTPg:HasVisibleTransparency` | no | no | no |
| `xmpTPg:HasVisibleOverprint` | no | no | no |
| `xmpTPg:SwatchGroups` | no | no | no |
| `xmpMM:OriginalDocumentID` | no | section 8.6 | by reference to Part 1 |

The Paged-Text schema defines five properties in every edition (`MaxPageSize`, `NPages`, `Fonts`,
`Colorants`, `PlateNames`). So three of the four are predefined under **no** reading, and the
fourth only under the reading section 1 sets aside. **The brief's hypothesis was the wrong way
round**: the properties are not in the edition part 2 names, and veraPDF does not read a later
one — it reads the same one, and agrees on all four. Agreement raises confidence; it decided
nothing here.

## 3. What the validator and the converter now say

- `pdf_archive`'s `needs_a_description`: a property needs an extension schema's description where
  its namespace has no predefined schema, **or** where the namespace's predefined schema does not
  define its local name. The first is described by naming its namespace, the second by naming the
  property itself (`described_properties`), since the namespace's own properties need nothing.
  `metadata/extension-schemas-embedded` reports both; `metadata/properties-use-known-schemas`
  still judges only what a predefined schema defines. `undescribed_schemas`, the converter's
  population, is the same function.
- The converter's existing container (ADR 1245) then describes them: `102952.pdf` → 2b under
  `as-shown` passes veraPDF 1.30.3, where HEAD's output failed on these four alone.
- A structured value (`SwatchGroups`) has no value type a container could name, and
  `undeterminable = "discard"` cuts it. That cut was not in the report; it now is, in the same list
  as every other property a conversion removes.

## 4. Left

Part 4 replaces this machinery with ISO 16684-2, which the tree does not hold; nothing here
reaches a part 4 target. Whether a description of a predefined namespace is an "extension schema"
in section 3's sense is a question the part does not answer; this reads it as one, as veraPDF does.
