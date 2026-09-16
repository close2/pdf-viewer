# 1151 — §14.7.5.4's array may be an object of its own, and reading it costs a page its widgets

Status: accepted and **built** in session 1154, which decided the completeness rule section 5 states.
Sessions 1151 (the reading and the measurements) and 1154 (the code).
Context: `crates/pdf-model/src/structure.rs` (`Tree::elements_on_page`, `Tree::stream_owners`,
`Tree::parent_tree_entry`), `crates/pdf-syntax/src/tree.rs` (`lookup_unresolved`),
`crates/viewer-core/src/accessibility.rs` (`nodes`, `gather`, `prune`),
`crates/viewer-core/tests/accessibility_census.rs`.
Builds: ADR 0325 (the page-scoped walk), ADR 0338, ADR 0488, ADR 0719, ADR 0970.
Clauses: ISO 32000-2 §7.3.10, §14.7.5.1.1, §14.7.5.3, §14.7.5.4 (Table 358, Table 359),
§14.8.4.7.2 (Table 368).

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

## 2. The population: 77 of the corpus's 109 documents with a structure tree

Counted over `doc/pdf.js/test/pdfs` and `doc/` by following each `/StructTreeRoot`'s `/ParentTree`,
descending its `/Kids`, and resolving every `/Nums` value: **77**, recounted in session 1154, where
session 1151's sweep followed no `/Kids` and said 75. It is the common spelling, not the rare one:
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

## 4. What shipping the parse alone would have cost: one file loses 226 of §12.7.5's controls

`prefilled_f1040.pdf` states 242 widget annotations, and 242 `Form` elements whose `/K` is an
`/OBJR` naming one. Not one of those annotations states Table 359's `/StructParent`, which that
table makes "[r]equired for all objects that are structural content items". So the parent tree
cannot name their elements, the page-scoped walk would never descend to them, and the answer loses
them: tracked `§12.7.5's controls` 272 → 46, `elements that are annotations` 415 → 189, `elements
reached` 4 060 → 3 648. `nodes`' existing guard cannot see it — it falls back when an element the
parent tree named was *not reached*, and says nothing about an element on the page the parent tree
never named.

`bug1978317.pdf` page 1 is the same disagreement mirrored: 32 768 `Link` elements, each named by
its annotation's `/StructParent`, none stating a `/K` of its own, so §14.7.5.3's "the object shall
be identified in the structure element's K entry by an object reference dictionary" is the half
this file omits. With the route working its page-scoped subtree is four times `MAX_NODES`, the walk
fills the bound before it can prune, and the answer is empty.

## 5. The completeness rule, decided: §14.7.5.3 is asked where Table 359's key is missing

§14.7.5.4 is the only route the standard states *from* a content item, and Table 359 makes the key
required of every structural content item — so the parent tree's answer for a page is complete only
where every such item states one. Where an object does not, the association still exists in the
*element's* direction, which §14.7.5.3 states and which needs no parent tree.

**It is asked for a widget annotation and for no other kind, and Table 368 is why.** That table
states one obligation about annotations and structure elements — "In a tagged PDF, Form shall be
used for each PDF widget annotation that belongs to the real content of the document" — while its
neighbours state permissions: `Annot` is "[e]ither an association … or a mechanism to include one
or more PDF annotations in the structure tree" and "[a]ll other annotation types may be referenced
by this structure element"; `Link` says what a `Link` element *means*, not that one exists. So a
link or a text annotation with no `/StructParent` is a file saying it is not a content item, and a
widget with none is a file that has broken a `shall` — which is what makes looking for its element
a recovery rather than an invention. `Tree::unkeyed_widget_owners` is the search, called from
`elements_on_page` as its sixth route, and it visits nothing on a page whose widgets state the key.

**Bounded, because finding an element by its content is the walk ADR 0325 took off a page turn.**
`MAX_OBJECT_SEARCH` is 8192 — `viewer-core`'s own bound on what one page's *answer* may hold, on
the argument that a recovery may not enter more elements than the answer may carry. The standard
states no number and this is a decision, not a derivation (trap 38). Measured: the eight corpus
documents with such a widget, and the two whose trees name an unkeyed object, hold at most 1922
elements and are searched whole in under 2 ms; ISO 32000-2's 37 such pages reach the bound in
8–10 ms instead of the 114 ms its 129 389-element tree takes to walk, and lose nothing, because no
`/OBJR` in it names an object that states no key.

**And the bound is reported rather than silent.** `accessibility_census` now tells the two empty
answers apart from the file: a parent tree naming at least `ANSWER_BOUND` elements for a page has
already exceeded what the answer may hold, so `bug1978317.pdf p1` is named in the at-bound class
instead of counted as a page whose answer failed. Both that class and the "names elements, answers
nothing" class are held by name now, the second holding `bug1365930.pdf p1` — whose `Document`
element states no `/K`, so §14.7.2's chain reaches neither of the two elements §14.7.5.4 names.

## 6. What it measured, built

Whole population, `accessibility_census`: elements reached 351 324 → **227 618**; placed 13 221 →
12 141; cells with headers 36 388 → 23 878; header associations 47 725 → 34 657. Placeless 130 171
→ 8 634. Unchanged: elements placed by their own marks 191 815, `/Alt` carried 670, controls 272,
elements that are annotations 11 258, caret 113 098, lines 199 568, characters 5 354 660, pages
that answer 2 463. Tracked: controls **272** and annotations **415** held — the 242 are kept — and
elements reached 4 060 → 4 059, the one being `pr20043.pdf`'s `Annot`, whose only content item is
an object the page's own `/Annots` does not list. Per document, exactly two moved:
`ISO-19444-1-2019-preview.pdf` 131 884 → 8 179 and that one element. No pixel moved
(`raster_golden` held 974, moved 0).
