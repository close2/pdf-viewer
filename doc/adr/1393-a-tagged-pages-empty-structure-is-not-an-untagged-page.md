# 1393 — A tagged page's empty structure is not an untagged page

Session 1278. Status: **accepted** and built.
Context: `crates/viewer-core/src/query.rs` (`PageStructure::tagging`, `Tagging`),
`crates/viewer-core/src/viewer.rs` (`accessibility`), `crates/viewer-accessibility/src/tree.rs`
(`unstructured`, `silence`), `crates/viewer-confined/src/protocol.rs` (`tagging_code`, `tagging_of`),
`crates/viewer-core/tests/{untagged_widgets,accessibility_census}.rs`.
Amends: ADR 0214 (what a page with no elements publishes), ADR 1381 section 3 (what it left).
Clauses: ISO 32000-2 §14.7.1, §14.7.2, §14.7.5.4, §14.8.1, §14.8.2.2.1, Table 353.

## 1. The reading

The brief cited §14.7.4 for a page's structure content; §14.7.4 is Namespaces. What reaches a page
is §14.7.5.4's parent tree, keyed by the page's `/StructParents`.

§14.7.2: "At the root of the hierarchy shall be a dictionary object called the structure tree root ,
located by means of the StructTreeRoot entry in the document catalog dictionary". Whether a document
states a logical structure is therefore one fact for all its pages, and ADR 0214's sentence ("this
document states no logical structure") is true only of a document without that entry. §14.8.1 adds
a second, stronger claim: "A tagged PDF document shall contain a mark information dictionary (see
"Table 353 -Entries in the mark information dictionary") with a value of true for the Marked entry",
and §14.8.2.2.1 says of such a document that "The document's logical structure encompasses all real
content". A page of it that the structure reaches nothing on either holds only artifacts or holds
real content its producer left out.

## 2. Decision

- `PageStructure::tagging` answers one of four: `Untagged` (no `/StructTreeRoot`), `Unread` (a
  tagged document's page not yet interpreted), `Unreached { marked }` (the structure reaches none
  of the page's content; `marked` is `/MarkInfo /Marked true`), `Reached` (the list holds it).
- The page node publishes a sentence per silence: ADR 0214's for `Untagged`; for `Unreached` that
  the document states a structure and none of it reaches this page, and with `marked` that the
  document declares itself tagged and that whatever on the page is not decoration the producer
  left out; for `Unread` that the page has not been read yet. Widgets follow each (ADRs 1369, 1381).
- The discriminant is `/StructTreeRoot`, not `/MarkInfo`: a structure without the tagged claim is
  still a structure, and a `/Marked true` catalog with no tree still states none.
- It crosses the confined wire as one byte after the two lists; an unknown byte is refused.

## 3. Measured

The census holds a page answered as the wrong one of the four at zero, and counts the unreached
pages: 44 tracked (3 in documents claiming `/Marked true`), 63 and 15 over the whole population.
No existing floor moved; the untagged count reads documents with no `/StructTreeRoot`.
