# 2014 — A structure type is read in the vocabulary of the namespace it ends in, and the default one is PDF 1.7's

Session 1589, slot 5 of batch ninety-three. Status: **accepted** and **built** in
`crates/pdf-model/src/structure.rs` (`StandardNamespace`, `StandardType::read_in`,
`StandardType::defined_in`, `Tree::standard_role`), `crates/viewer-accessibility/src/role.rs` and
`crates/pdf-archive/src/table/document_level.rs`. Context: round 1577's finding (`doc/history/1577`,
"Unfinished"); ADR 0253 (Errata Collection 3's sentence for §14.8.6.1); ADR 1990 section 7 (the part 2
reading); ADR 1948 (the copy's logical order).

## 1. What the standard says

§14.8.6.1 names two standard structure namespaces and makes the older the default:

> When a namespace is not explicitly specified for a given structure element or attribute, it shall
> be assumed to be within this default standard structure namespace.

and the default is PDF 1.7's, whose schema the same subclause says is ISO 32000-1's clause 14.8.
Annex M lists the difference: eleven types the PDF 1.7 namespace defines and PDF 2.0's does not
(`Art`, `BlockQuote`, `TOC`, `TOCI`, `Index`, `Private`, `Quote`, `Note`, `Reference`, `BibEntry`,
`Code`), and nine the other way (`DocumentFragment`, `Aside`, `Hn` past six, `Title`, `FENote`,
`Sub`, `Em`, `Strong`, `Artifact`). Errata Collection 3 adds that the type an element with no stated
namespace ends at, after its role map, shall be one the default namespace defines (ADR 0253).

`Tree::standard_role` followed §14.8.6.2's walk and then read the name it ended at in PDF 2.0's
vocabulary whatever the namespace, so the eleven never read as standard types — the most common
names in the corpus's tagged documents that a screen reader was told nothing defines.

## 2. What is built

`StandardType` gains the eleven, with the categories ISO 32000-1 gives them: grouping for the six
of its Table 333, inline for the five of its Table 338. `StandardNamespace` names the two
namespaces; `StandardType::defined_in` is Annex M's two lists (and `since_pdf_2_0` now counts `Hn`
past six, which ISO 32000-1's Table 334 does not define); `StandardType::read_in` reads a name in one
namespace; `StandardType::read`, for a caller holding a name and no namespace, reads either.
`Tree::standard_role` reads the name in the namespace it ends in: an explicit standard namespace
in its own vocabulary, held to; no namespace in PDF 1.7's. `viewer-accessibility` maps the eleven
(`Art` → `Article`, `BlockQuote` → `Blockquote`, `TOC` → `DocToc`, `TOCI` → `ListItem`, `Index` →
`DocIndex`, `Note` → `DocFootnote`, `BibEntry` → `DocBiblioEntry`, `Code` → `Code`, `Quote` and
`Reference` → `Label`, `Private` → `GenericContainer` and not spoken), each loss in the module's table.
`pdf_archive::standard_structure_element` becomes `standard_role`'s answer and its own copy of
Annex M's list goes, since the reading it supplied is now `pdf-model`'s.

## 3. A PDF 2.0 type stating no namespace is read as PDF 2.0's — a choice

An `Em`, a `Title` or an `Artifact` with no `/NS` breaks Errata Collection 3's `shall`; what a reader
does with it the standard does not say. Two readings were open. **Refuse** — the name is not a type —
and an `Artifact` element's line numbers go into a copy and an `H7` stops being a heading.
**Recover** — the name is defined by exactly one standard namespace and the file stated no other —
and the producer's evident meaning stands. This takes the second: the default namespace exists "[t]o
facilitate conversion of documents created against versions of the PDF standard earlier than PDF
2.0", which a PDF 2.0 type cannot be, and this tree's two other readers of the question already
recovered — `viewer-core`'s `tagged_copy` fixture (an `Artifact` element with no `/NS`, kept out of
the copy) and `pdf-archive`'s part 2 leniency. An **explicit** namespace is a statement and is held
to: an `Em` naming the PDF 1.7 namespace is no type. The choice is a number rather than a silence:
`viewer-core --test accessibility_census` prints "PDF 2.0 types stating no namespace, read as PDF
2.0's" by name.

## 4. What it moved

`viewer-core --test accessibility_census`, the same export with and without the change (HEAD plus
the census's two new counts, then plus the change): elements a host is told no standard namespace
defines, 2 548 on 1 310 tagged pages → 31 on 5, the 31 being MathML's own names and two foreign ones
(`Article`, `Subtitle`); every other count of the census identical. `TOC`, `TOCI`, `Art`,
`BlockQuote`, `Index` and `Private` now end a block in `Tree::logical_range`; `Note`, `Quote`,
`Reference`, `BibEntry` and `Code` are inline and end nothing, as a name nobody defined did. Over
every page of the pdf.js corpus's tagged documents and the first five of each specification in
`doc/` (206 pages copied in logical order), 17 pages — contents pages — move from 52 block ends to
443, and the copy that `viewer-core`'s `logical_copy` would make of them differs on 10 pages only by
the trailing space each contents line loses: the readback had already put a line break between the
entries. No drawing reads a structure type, and the six arms say so: every page digest at 1× and
4× on the three lanes equals HEAD's.

## 5. What it does not do

- **`Private`'s descendants are still presented.** ISO 32000-1 Table 333 says neither the element
  nor any of its descendants is to be interpreted or exported; leaving them out is `viewer-core`'s
  walk, and no corpus document states one.
- **A host reads the name alone.** `viewer_core::AccessibilityNode::role` crosses as a string (ADR
  0214), so `viewer-accessibility` reads it with `StandardType::read` and a foreign namespace's
  homonym of a standard name maps as the standard type there, where `Tree::standard_role` refuses it.
  Carrying the standard type across is `viewer-core`'s field to add.
