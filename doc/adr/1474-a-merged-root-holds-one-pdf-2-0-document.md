# 1474 — A merged root holds one PDF 2.0 `Document`

Session 1319. Status: **accepted**.
Context: ISO 32000-2 Annex L (Tables L.1, L.2), §14.7.2, §14.8.4.3 (Table 364), §14.8.6.2;
`crates/pdf-transform/src/structure.rs` (`Carry::wrap_documents`, `pdf_2_0_namespace`,
`wrapper_element`), `crates/pdf-model/src/structure.rs` (`Tree::in_pdf_2_0_namespace`). Carries out
what ADR 1461 priced for Annex L.

## 1. The reading

Annex L: "Elements in the standard structure namespace for PDF 2.0 shall not have child or parent
elements in the standard structure namespace for PDF 2.0 that are not explicitly listed in Table
L.2", and "The containment rules specified in Table L.2 shall also apply to structure elements that
are role mapped into the standard structure namespace for PDF 2.0". Table L.2's first row gives
`StructTreeRoot` the child `Document` at an occurrence of `1`. Table L.1 defines `0..1`, `0..n` and
`1..n` but not a bare `1`; beside those the bare `1` reads as exactly one, which is ADR 1461's reading
too. So a merge of two sources whose roots each hold a PDF 2.0 `Document` would write a root that
breaks the table.

## 2. The construction

Where the root's top-level elements include PDF 2.0 `Document`s from **more than one source**, they
are written inside one new `Document` in that namespace. Its `/P` is the root, its `/K` holds them in
source order, and each one's `/P` names it. Table L.2 lists `Document` among a `Document`'s children
at `0..n`, and Table 364's EXAMPLE 2 is this shape: "the PDF at the top level is one document
containing several documents". The wrapper states only Table 355's required entries and `/NS`; a
`/Lang` or `/T` would be a claim nobody made. Its `/NS` reuses a PDF 2.0 namespace dictionary a
source's root already lists in `/Namespaces`. Where none does, one is written and added to the array,
because §14.8.6.2 requires an explicit namespace to be listed there.

The population is the annex's own: `Tree::in_pdf_2_0_namespace` asks where §14.8.6.2's role map
*ends*, so a role-mapped `Document` counts, and an element in PDF 1.7's default namespace does not.

## 3. Two things not done, and why

- **The carried elements are not re-typed `DocumentFragment`.** The type is defined for "a portion of
  content that constitutes -or is intended or perceived as -just a part of a logical document", and
  Table 364's EXAMPLE 3 includes "extracts from original documents are concatenated into a new
  document". That fits a merge of page selections. But whether a source's element still encloses a
  whole logical document is a statement about the content, and the producer made it. Every other
  element crosses as its producer typed it (ADR 0834), and Table L.2 admits `Document` under
  `Document` with no condition. So re-typing is not needed for conformance, and it would be this
  program's judgement written over the producer's.
- **One source's own violation is carried as written.** A source whose root already holds two PDF 2.0
  `Document`s broke the annex in its own file. That is ADR 0821 section 2's distinction, the one
  `/ID` collisions already follow: the writer answers for what the derivation creates.

## 4. Tests

`tests/merge.rs::two_pdf_2_0_documents_are_written_inside_one` reads the merged root back (one
child, a PDF 2.0 `Document` whose two children are the sources' `Document`s naming it as `/P`) and
holds it to `support::check_structure`, to `pdf_archive`'s `logical-structure/structure-tree-root`
and `logical-structure/role-map-terminates-at-a-standard-type`, and to `qpdf --check`.
`documents_outside_the_pdf_2_0_namespace_are_not_wrapped` is the twin for PDF 1.7 elements.
