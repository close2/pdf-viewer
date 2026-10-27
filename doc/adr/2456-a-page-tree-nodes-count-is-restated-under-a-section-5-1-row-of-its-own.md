# 2456 — A page tree node's `/Count` is restated under a section 5.1 row of its own

Status: implemented. Session 1811, the archive slot of batch one hundred and twenty-one that
takes up ADR 2444's named route. Context: ADR 0947's second rule, ADR 2444, `doc/todo/72`.

## The reading

ISO 32000-2 §7.7.3.2's Table 30 states `/Kids` and `/Count` as required entries of every page
tree node and states their types, array and integer; `/Count` is "( Required ) The number of leaf
nodes (page objects) that are descendants of this node within the page tree.", and its NOTE puts
the consistency on the writer: "A PDF writer shall ensure that the value of the Count key is
consistent with the number of entries in the Kids array and its descendants which definitively
determines the number of descendant pages." ISO 32000-1:2008's Table 29, the base document of
ISO 19005-2, states the same two rows without the NOTE. ISO 19005's section 5.1, in both parts,
makes adherence to the base standard a requirement, and the validator's
`conformance/adheres-to-the-base-standard` is `Unchecked`, so until now no requirement failed on a
broken node and ADR 0947's second rule forbade the restatement ADR 2444 named.

## The decision

Two implemented rows under section 5.1 of both parts, each narrowed to presence and type, beside
`file-structure/hexadecimal-string-holds-only-digits` and carried by both parts' 5.1 reading:

- `file-structure/page-tree-node-count-is-an-integer` — a node whose `/Count` is missing or
  resolves to no integer, named by its object. Answered `Mechanical` by
  `Rewrite::PageCountRestated`: the node states the number of leaves `pdf_model::Pages` walks
  beneath it (`archive::page_tree::restated`, the reader's leaf count entry by entry — cycles
  stepped over, both bounds the reader's), so the output states the pages this reader already
  drew. A `/Count` that is an integer and disagrees with the leaves is not this row's.
- `file-structure/page-tree-node-kids-is-an-array` — a node whose `/Kids` is missing or no array.
  Refused by name (`the-fence`): the tree states no child, and the verb refuses such a document
  before stage 1 as ADR 2444 does; the pre-stage check is now held to `/Kids` alone.

Three shapes keep the count refused, each with its sentence: a failing node written inside its
parent's `/Kids` (no object to restate at, `not-built-yet`), one reached by two routes with a
different number of leaves beneath each (Table 30's "A page tree shall not contain multiple
indirect references to the same page tree node." is about it; `the-fence`), and one past the
walk's bounds.

## Consequences

- `poppler-91414-0-53` and `-54` (pdf.js) and `T02-02_008_page-tree-node-no-count` (`doc/corpora`)
  convert at 2b and at part 4 under both shipped profiles, and veraPDF finds every output
  compliant. `PDFBOX-6040-nodeloop`'s count is answered and it stays refused, now by
  `file-structure/file-header`, since its header is no version this tree reads.
- Each freed output draws as an output of the same document with an integer `/Count` planted in
  place: the restatement moves no pixel. The fourteen edge pixels between `poppler-91414-0-53`
  as the reader draws it and its conversion are the substitution of its unembedded Helvetica,
  drawn there with `1 Tr`, and the plant shows the same fourteen.
- The validator now reports a broken node at every target, which `check` users see.
