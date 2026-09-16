# 1151 — §14.7.5.4's array may be an object of its own, and reading it costs a page its widgets

Status: accepted — the reading. **Not implemented**: section 4 is why, and it is a round's work.
Session 1151.
Context: `crates/pdf-model/src/structure.rs` (`Tree::elements_on_page`, `Tree::stream_owners`,
`Tree::parent_tree_entry`), `crates/pdf-syntax/src/tree.rs` (`lookup_unresolved`),
`crates/viewer-core/src/accessibility.rs` (`nodes`, `gather`, `prune`),
`crates/viewer-core/tests/accessibility_census.rs`.
Builds: ADR 0325 (the page-scoped walk), ADR 0338, ADR 0488, ADR 0719, ADR 0970.
Clauses: ISO 32000-2 §7.3.10, §14.7.5.1.1, §14.7.5.3, §14.7.5.4 (Table 358, Table 359).

## 1. The reading

§14.7.5.4: "For a content stream containing marked-content sequences that are content items, the
value shall be an array of indirect references to the sequences' parent structure elements." The
array is the *value*, and §7.3.10 says what may be done with any value: "Any object in a PDF file
may be labelled as an indirect object." §14.7.5.4's own EXAMPLE 2 writes it that way —
`/Nums [0 101 0 R 1 102 0 R … 6 [1 0 R] …]` — so a reference to an array and a direct array are
two spellings of one entry.

`Tree::parent_tree_entry` answers **unresolved**, which is right for the object form, whose value
*is* a reference and whose whole content is identity. Both array call sites then pattern-match
`Object::Array` on that unresolved value, so a file that wrote the array indirectly is read as
having stated nothing. `elements_on_page` returns `None`, which its caller reports as "the page
states no `/StructParents`" — false of every such page.

## 2. The population: 75 of the corpus's 109 documents with a structure tree

Counted over `doc/pdf.js/test/pdfs` and `doc/` by following each `/StructTreeRoot`'s `/ParentTree`
and resolving every `/Nums` value. It is the common spelling, not the rare one:
`structure_simple.pdf` — the smallest tagged corpus document, and the census's own pinned
fixture — writes `20 0 obj <</Nums[0 19 0 R]/Type/ParentTree>>`. So ADR 0325's page-scoped walk,
which exists so that a page's answer costs the page rather than the document, has never run on
69% of the tagged corpus; every one of those pages takes the whole-tree fallback instead.

## 3. What reading it correctly buys

Measured A/B over the census, the two arms back to back (whole population): elements reached
351 324 → 227 207. Nothing that carries content moved — elements placed by their own marks
191 815, elements a caret reaches 113 098, lines 199 568, characters 5 354 660, all unchanged. The
whole 121 537 of the drop is elements **with no place by any route**: 130 171 → 8 634. They were
one document's tree counted once per page — `ISO-19444-1-2019-preview.pdf` answered 131 884
elements over 19 pages and answers 8 179 with the route working, because its elements state no
`/Pg` and the fallback keeps an element for every page that cannot rule it out.

## 4. Why it is not implemented here: one file loses 226 of §12.7.5's controls

`prefilled_f1040.pdf` states 242 widget annotations, and 242 `Form` elements whose `/K` is an
`/OBJR` naming one. Not one of those annotations states Table 359's `/StructParent`, which that
table makes "[r]equired for all objects that are structural content items". So the parent tree
cannot name their elements, the page-scoped walk never descends to them, and the answer loses them:
tracked `§12.7.5's controls` 272 → 46, `elements that are annotations` 415 → 189, `elements
reached` 4 060 → 3 648. `nodes`' existing guard cannot see it — it falls back when an element the
parent tree named was *not reached*, and says nothing about an element on the page the parent tree
never named.

`bug1978317.pdf` page 1 is the same disagreement mirrored: 32 768 `Link` elements, each named by
its annotation's `/StructParent`, none stating a `/K` of its own, so §14.7.5.3's "the object shall
be identified in the structure element's K entry by an object reference dictionary" is the half
this file omits. With the route working the census names the page in its defect class, correctly —
and a full answer would put 32 768 nodes where `viewer-core`'s bound admits 8 192.

So the parse is right and shipping it alone makes the program worse on a real file. What it needs
is a decision about ADR 0325's page-scoped walk against the completeness §14.7.5.4 requires and
real files do not deliver, plus the node bound above — an argued round, not a hunk. What this
session did ship is the diagnosis: the census now says which of the two silences each page is.
