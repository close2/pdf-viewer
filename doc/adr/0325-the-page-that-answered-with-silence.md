# ADR 0325 — The page that answered with silence, and the tree that was walked to find it

Status: accepted, 2026-08-14. Session 490. Amends §14.7's, §14.7.2's and §14.7.5.4's ledger rows.
Extends ADR 0214's bridge; changes nothing it decided.

## The question

`doc/todo/31` recorded, without looking into it, that "[t]he answer for any page but the first of a
large tagged document is empty". It is worse than the sentence sounds. `Query::AccessibilityTree`
is what a screen reader asks on every page turn, and on ISO 32000-2 — 1023 tagged pages — the
answer past the first hundred or so pages is an empty list, which is **exactly the answer an
untagged page gives**. A person using a screen reader is told that a tagged document says nothing
about itself, and no report, no count and no gate says otherwise.

`viewer-core --example accessibility_cost <file> <page>` is the instrument, and it prints the
defect in one line: the page the document opens on answers with 17 nodes, its page 400 with none.
Pages 1, 2, 40, 60 and 100 answer; 150, 200, 400 and 800 do not.

## The diagnosis, which is not the bound

`viewer_core::accessibility::nodes` walked §14.7's structure tree **from the structure tree root**,
gathering every element in the document, and pruned to the page afterwards. `MAX_NODES` — 8192,
documented as "how many nodes one page's tree may hold" — stopped that walk. So the bound written
for one page's *answer* was being spent on the whole document's *tree*, and it ran out among the
first hundred pages' elements. Everything after them was never reached, and pruning something that
was never gathered produces an empty list.

Raising the bound is the fix that looks obvious and is wrong twice over: it leaves the cost of
every page turn proportional to the size of the document rather than of the page, and it leaves the
same cliff one order of magnitude further along. The defect is the *route*.

## What the standard states

§14.7.5.4 is titled "Finding structure elements from content items", and it exists because the
obvious route does not:

> Because a stream cannot contain object references, there is no way for content items that are
> marked-content sequences to refer directly back to their parent structure elements (the ones to
> which they belong as content items). Instead, a different mechanism, the structural parent tree ,
> shall be provided for this purpose. For consistency, content items that are entire PDF objects,
> such as XObjects, shall also use the parent tree to refer to their parent structure elements.

The key is the page's own, and Table 359 makes it required of the content stream:

> **StructParents** — (*Required for all content streams containing marked-content sequences that
> are structural content items; PDF 1.3*) The integer key of this object's entry in the structural
> parent tree.

and the value is the answer, in two forms:

> For an object identified as a content item by means of an object reference …, the value shall be
> an indirect reference to the parent structure element.
>
> For a content stream containing marked-content sequences that are content items, the value shall
> be an array of indirect references to the sequences' parent structure elements. The array element
> corresponding to each sequence shall be found by using the sequence's marked-content identifier as
> a zero-based index into the array.

So the document has already written down which elements a page's content belongs to. Table 355's
`/P` — "(*Required*; shall be an indirect reference) The structure element that is the immediate
parent of this one in the structure hierarchy" — turns that set into the *subtree* the page
occupies, by naming everything above each of them.

`pdf-model` has read the parent tree since the seventy-eighth session, for the other direction: one
marked-content identifier to its element, which is what text extraction and §14.9.4's replacement
text need. What it had never been asked is the whole page at once.

## Decision 1 — reach the page's elements through §14.7.5.4, and keep the order §14.8.2.5 states

Two new readings in `pdf_model::structure::Tree`:

- **`elements_on_page`** asks §14.7.5.4 for the elements a page's content items name. All three
  kinds the clause distinguishes are asked, because each is keyed differently and a reader that
  found two of the three would look right on nearly every file: the page's own `/StructParents`
  array for its marked-content sequences, each annotation's `/StructParent` for §14.7.5.3's object
  references, and each `XObject`'s — which is the clause's own EXAMPLE 1, a form `XObject` stating
  `/StructParent 6`. A form's resources are followed for the `XObject`s drawn inside it.
- **`ancestry`** follows Table 355's `/P` up from each of them, bounded, stopping at Table 354's
  root.

`viewer_core::accessibility::nodes` then walks **down from the root as before** and steps over
every child outside that set. The direction of the walk is deliberately unchanged, and the reason
is the order: §14.8.2.5.1 defines logical content order as "a depth-first traversal of the
document's logical structure hierarch y", and a set of elements taken out of the parent tree is in
*marked-content identifier* order, which is the content stream's. Walking down keeps the tree's own
order, which is the entire reason a tagged document is worth reading; the parent tree decides only
which branches are entered.

**Identity, not equality, is what the walk needs**, and that is why `pdf-syntax` gained
`tree::lookup_unresolved`. `tree::lookup` resolves the value it finds, which every other consumer
of a name or number tree wants; §14.7.5.4's values *are* references, and resolving one throws away
the only thing that distinguishes an element from an equal dictionary elsewhere in the file.
`Tree::identified_children` — which `Tree::descend` already had for its own cycle guard — became
public for the same reason.

## Decision 2 — the pruning stops at a table, on §14.8.5.7's own grounds

A `TH` that states no `/Scope` gets its axis from its place in the table's grid (ADR 0300), and a
grid is counted from the table's first row. A table continued from the page before has its rows on
two pages, so a walk that entered only *this* page's rows would number them from zero and announce
every continuation row's header as a column's. So the walk descends into a `Table` element's whole
subtree whatever the page said, and the cells belonging to other pages are dropped afterwards by
the same pruning that always dropped them. The measured evidence is that this is not a theoretical
concern and that it works: page 40 of ISO 32000-2 answers with 69 nodes, 17 of them carrying
headers and 25 header associations in all, before and after this change, to the number.

## Decision 3 — three ways for the file to have not answered, and what each gets

The parent tree is a *second* statement a document makes about itself, and nothing checks it
against the first. Three cases, each decided rather than left to chance:

- **The page states no `/StructParents`**, or the key it states is not in the tree, or its entry is
  not an array. Then §14.7.5.4 has not answered for this page's sequences and nothing may be
  concluded from what it *did* answer — so the whole tree is walked, exactly as before.
  `elements_on_page` returns `Option` rather than a set for this reason, and the distinction is not
  academic: **the corpus A/B below caught the first version of this change getting it wrong.** A
  document whose pages carry widget annotations and no `/StructParents` produced a complete-looking
  answer built from the annotations' elements alone, with every paragraph on the page missing.
- **`/K` downwards and `/P` upwards disagree**, so an element the parent tree named is not where its
  own ancestry says it is and the pruned walk stepped over it. The walk records which elements it
  reached; where one the parent tree named is not among them, the whole-tree walk is run instead.
- **The bound was reached anyway.** Then the disagreement above cannot be trusted — a walk cut short
  did not reach things for a different reason — and the answer stands as it is.

An over-generous set costs nothing and is therefore preferred wherever the clause leaves room: an
`XObject` a page's resources name may never be drawn, and an element kept for it is dropped again
unless one of its content items states this page in Table 355's or Table 358's `/Pg`. Missing an
element is the only error with a cost.

## What was measured

**A corpus A/B, and it is the reason two of the three cases above are written down.** The pruned
walk and the whole-tree walk were run side by side on every page sampled from every document in
`doc/pdf.js/test/pdfs` and `doc/`, six pages per document, and required to produce **the same
gathered elements, in the same order, with the same identifiers, parents, phrases, languages,
header scopes, bounds and header associations** — the `Gathered` value in full, not a count. Pages
whose whole-tree walk was itself bounded out are skipped, because there the two *should* differ and
the difference is the defect.

The first run failed on the fifteenth document, which is what a comparison is for.

**And the node counts on the pages that already worked are unchanged**, which is the same claim one
level up: ISO 32000-2's pages 1, 2, 40, 60, 100 and `Well-Tagged-PDF-WTPDF-1.0.pdf`'s page 20
answer with the same number of nodes, bounds, headed cells and associations as before. Pages 150,
400, 800 and 1022 answered with nothing and now answer with 69, 28, 39 and 30 nodes.

**And it was verified on a real accessibility bus**, which is what the defect was actually about:
`doc/verify.md`'s recipe — a session bus, at-spi's own bus and registry, `Xvfb`, and `busctl`
walking `org.a11y.atspi.Accessible` from the registry root, a real client rather than this
program's own types — with `pdf-viewer --page 400 doc/ISO_32000-2_sponsored_EC3.pdf`. The client
walks `main` → `pdf-viewer` → the frame → the document → **`page 385 (400 of 1023)`**, which is
§12.4.2's own label, and then into the page's elements: the paragraphs of §8.4.3.6 and a `Figure`
announced with §14.9.3's `/Alt`, *"Figure illustrating the consequences of rasterization with…"*.
Before this change that page's subtree was empty on the bus, because the answer behind it was.

**No wall-clock figure is recorded here.** The change removes a walk of the document from a
question asked per page turn, and the direction is not in doubt; the number is
`doc/todo/31`'s cost item to take, with the callgrind method ADR 0312 established, because a
stopwatch on a machine running ten rounds measures the machine.

## What this does not fix

- **A page of a *large* document that states no `/StructParents` is still empty**, because the
  fallback is the walk that had the defect. That is the honest answer rather than a gap: without
  the entry Table 359 requires, the file has stated no route from its page to its elements, and a
  bounded walk is what remains. Nothing in the corpus is in that position — the A/B would have
  found it.
- **A `/StructParents` array shorter than the page's marked-content sequences** loses the elements
  it does not name. The clause makes the array "one for each marked-content sequence contained
  within that content stream", so this is a malformed file, and the reached-set check does not see
  it because the missing element was never claimed.
- **The cost item and `MAX_NODES` itself** are untouched. The bound now bounds what its own comment
  says it bounds, which is the point.

## Alternatives rejected

- **Raise `MAX_NODES`.** Moves the cliff and keeps the cost proportional to the document. The bound
  was never the defect.
- **Build the answer *out* of the parent tree, bottom-up.** It is one lookup and no walk, and it
  loses §14.8.2.5's order — the marked-content identifiers are the content stream's sequence, which
  is the order a tagged document exists to override.
- **Skip a child before resolving it, by its reference alone.** It is the obvious way to make the
  pruned walk cheaper still, and it is unsound: §14.7.5.1.1's content items may themselves be
  indirect objects, and one skipped for not being in the keep set would silently take its
  annotation off the page. Rejected on principle 1, and recorded here so that the next round to
  look at the cost knows it was considered.
