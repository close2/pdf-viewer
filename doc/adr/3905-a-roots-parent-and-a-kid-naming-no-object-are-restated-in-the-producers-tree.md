# 3905 — A root's `/Parent` and a kid naming no object are restated in the producer's tree

Status: implemented, in `pdf-transform` (`archive::page_tree::restated_nodes`, the rewrite
`page-tree-restated`, and `archive::decision`'s two rows `file-structure/page-tree-root-states-no-parent`
and `file-structure/page-tree-kids-name-dictionaries`). Session 2301, `doc/todo/72`'s top site.
Context: ISO 32000-2 §7.7.3.2 Table 30 and §7.3.10; ISO 19005-2 and ISO 19005-4 section 5.1; ADR 2558
item 5, ADR 2583, ADR 2596, `doc/questions/A727`.

## The gap

`archive_unbuilt` ranked `file-structure/page-tree-kids-name-dictionaries` first among the sites marked
`not-built-yet` at every target: 9 pdf.js and `doc/corpora` documents. The ninth row of the same table,
`…-root-states-no-parent`, had 1 (`PDFBOX-6040-nodeloop.pdf`). Both rows were refused by name for a fault at
a node written directly, which the conversion's walk does not follow (ADR 2583 item 5). Every source the walk
did reach was declined before stage 1. It converted only behind `recovered-structure`, as a new root over
the pages the reader drew (ADR 2558). That means a profile has to accept the loss of the producer's tree for
faults that remove nothing a reader sees. Before this change, no witness converted without a profile. Under
`as-shown`, 5 converted, all veraPDF-compliant at 2b and 4: both `PDF-HUL-29` files, `T02-02_003`,
`T02-02_004` and PDFBOX-6040. Four stopped at `page-not-held`.

## Decision

1. **Restated, not rebuilt.** Table 30's `/Parent` row says the entry is "not permitted in the root node".
   A root without it states what the row defines, and §7.7.3.1 already makes the catalog's `/Pages` the
   root. A `/Kids` entry that names no object is a direct value that is no dictionary, or a reference that
   §7.3.10 makes the null object. It is neither of the children the row allows. The arrays "definitively
   determine" the descendant pages (the row's NOTE), so the pages beneath every node are the same without
   that entry. `A727` already said "dropping such a kid changes nothing a reader shows". So the entry is
   removed and the root's `/Parent` is removed. This is a `Mechanical` answer that needs no loss, and the
   producer's nodes, `/Count`s and inherited entries are kept. Where a `/Count` counted the entry, ADR
   2819's restatement corrects it from the leaves.
2. **What still declines.** A reference to an object that the cross-reference information places, but that
   this reader reads as no dictionary, is a page the producer wrote and the file does not hold. It still
   declines the source, and `page-not-held` answers it (ADR 2596). The conversion's walk declines that shape
   and the one exception below. One classifier, `page_tree::child`, separates the three kinds of entry for the walk,
   for `children_not_held` and for the restatement, so the three cannot disagree. **One exception**: an
   entry naming no object beside a sibling dictionary that is neither a page (`/Type /Page`) nor a node
   (`/Kids`, or `/Type /Pages`) still declines the source. Removing the entry would leave a child the
   row does not allow, and the rebuild already writes that tree conforming (`T02-02_003`, whose second
   kid is the page's resource dictionary). The first probe of this change, which had no exception,
   refused that witness at `conformance/adheres-to-the-base-standard` (a substituted media box) where
   `as-shown` had converted it.
3. **Written at the node's own object.** A node written inside the catalog or inside its parent's `/Kids`
   has no object to be restated at. It keeps a `NotBuiltYet` sentence, the same limit as `/Count` (ADR
   2456), and no corpus document has that shape.

## Measured

The 9 witnesses were converted at 2b and 4 with no profile and under `as-shown`, and each output was
checked by veraPDF 1.30.3. The counts before and after are in the round's record
(`doc/history/2301-*.md`). The census after the change lists neither row as not built.

## Rejected

- **Keep the decline and widen the walk to nodes written directly.** That would leave every witness
  depending on `recovered-structure`, a loss that a statement answers.
- **Remove a placed-but-unread kid too.** That page is one the producer wrote. Removing it without the
  stated count is the silent loss that ADR 2596 exists to prevent.
