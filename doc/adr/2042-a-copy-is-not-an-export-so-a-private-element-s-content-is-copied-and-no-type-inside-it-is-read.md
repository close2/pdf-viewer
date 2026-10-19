# 2042 — A copy is not an export, so a `Private` element's content is copied and no type inside it is read

Session 1603, slot 5 of batch ninety-five, a clause round. Status: **accepted** and **built** in
`crates/pdf-model/src/structure.rs` (`Tree::logical_range`'s element arm, the `private` flag of its
`Open`) and `crates/viewer-core/src/select.rs` (`logical_copy`'s statement of what it is given).
Context: ADR 2028 section 2, which withheld the element from the accessibility tree and left this
question open; ADRs 0444, 1948, 1970. Test: `crates/viewer-core/tests/tagged_copy.rs`'s
`a_private_element_is_copied_and_its_types_are_not_read`.

## 1. The question, and the two sentences it turns on

`Private` is defined only by the PDF 1.7 namespace, whose schema §14.8.6.1 names as ISO 32000-1's
clause 14.8 (Annex M lists it among the eleven types PDF 2.0 dropped; ISO 32000-2 has no row for it,
and its Table 364 is the document-level types). ISO 32000-1 Table 333 says that neither the element
nor any of its descendants shall be interpreted or exported to other document formats (the sentence
is quoted, as that edition prints it, beside the code). It forbids two things, and `Tree::logical_range` — the copy in §14.8.2.5.1's logical order — touched
both questions: it read every type below the element (a `TD` ended a cell, a `P` a block) and copied
the subtree's text.

**A copy to the clipboard is not an export to another document format, and the standard says so in
the clause the sentence sits in.** §14.8.1's NOTE, in both editions, lists the operations tagged PDF
is for, and pasting and conversion are separate items: "Extraction of text and graphics for pasting
into other applications" and "Conversion to other common file formats (such as HTML, XML, and RTF)
with document structure preserved". Table 333's "exported to other document formats" is the second.
ISO TS 32005 Table 2, which restates `Private` for a document that uses both namespaces, keeps the
two apart as well — it names consuming the content and exporting to other document formats as two
occasions — and makes ignoring the subtree on either a permission rather than an obligation. So the
brief's hypothesis, that a copy reproduces content in another application and is therefore the
export, did not hold: the standard's own vocabulary names that operation and calls it something else.

**The first half binds the copy.** A logical copy *interprets* the structure — the types decide where
a block ends and where a cell does — and Table 333 forbids that for the element and everything below
it. *Interpreted* is the type's meaning, not the element's place: the same table says of `NonStruct`
that it shall not be interpreted or exported to other document formats and that its descendants shall
nevertheless be processed normally, so an element that is not interpreted still stands in the
traversal its children are reached through. §14.8.2.5.1's traversal is §14.7's hierarchy, and orders the
subtree; it does not filter it.

## 2. What is built

In `logical_range`, an element that is `Private`, or below one, is read as an element of no type —
what the walk already does with a type it does not know: it ends with `Boundary::Flow` and passes its
parent's category through. So inside the subtree no block or cell ends, no element is an `Artifact`,
and the readback's own separators divide its content; the boundary *before* it is still whatever the
element before it ended with, because that element is outside it. A marked-content artifact
(§14.8.2.2.2's `/Artifact` property list) is not in the structure tree and is still declared. The
content is copied at its place in the traversal, which is where the page order's copy — what a host
falls back to when `logical_range` answers `None` — already put it, so the two orders now agree that
the person who selected the ink gets it. On the fixture, a two-cell table and a paragraph inside a
`Private` element copy as `C1 D1\nKept.` where they copied as `C1\tD1\nKept.`.

## 3. What it does not change, and why the accessibility tree differs

`logical_text` (the page's text in logical order) reads no types and is unchanged; the drag's
highlight is the ink's and unchanged. ADR 2028's accessibility walk still presents neither the
element nor its descendants: a platform tree is the structure *presented*, a node and a role per
element, so every part of it is an interpretation, where a copy is the person's selection of a
readback that exists without the structure. No corpus page holds a `Private` element (ADR 2028's
census over 3 882 files), so the fixture is the evidence and no copy of a corpus page moves.
