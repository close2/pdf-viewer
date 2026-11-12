# 2819 — An integer `/Count` other than the leaves is the writer's to correct

Round 2000, the audit-fix round (prose item 21 of `doc/reviews/1983-specification-audit.md`, its
"Table inventories and ADR assertions" section).
Status: **accepted and built** in `pdf-archive` (the row
`file-structure/page-tree-node-count-counts-its-leaves`) and `pdf-transform`
(`archive/page_tree.rs`: `restated`, and the refusal `optimize` and `redact` share).
It supersedes ADR 2456 for one point: which `/Count` values the conversion restates.
Context: ADRs 0947, 2444, 2456, 2596, 2670.
Clauses: ISO 32000-2 §7.7.3.2 (Table 30 `/Count` and the NOTE under the table); ISO 19005-2 and
ISO 19005-4 section 5.1.

## The finding

ADR 2456 restated a `/Count` that is missing or no integer, and carried every integer unchanged. The
audit: "The Table 30 count requirement is general; ADR 2456's narrow archive repair does not narrow
that standard requirement. Its noninteger-count remedy also does not prove correctness of every
integer Count." That holds. Table 30 defines the entry as "( Required ) The number of leaf nodes
(page objects) that are descendants of this node within the page tree.", and the NOTE under it reads:

> A PDF writer shall ensure that the value of the Count key is consistent with the number of entries
> in the Kids array and its descendants which definitively determines the number of descendant pages.

The conversion is a PDF writer. An integer that disagrees with the leaves is as inconsistent as a
string, and nothing in the NOTE distinguishes them.

## The decision

1. **A second section 5.1 row**, `file-structure/page-tree-node-count-counts-its-leaves`, fails every
   node whose integer `/Count` differs from the leaves its `/Kids` reach, counted the way
   `pdf_model::Pages` counts them where it has no count to trust. The integer row keeps its own
   question, so a report names which of the two a file broke.
2. **The conversion answers it with the same rewrite**, `PageCountRestated`: the leaves are written
   at the node. The NOTE makes the `/Kids` arrays definitive, so where this reader trusted an
   under-counting integer and drew fewer pages than the tree holds, the archive holds every page the
   tree names — the producer's tree, not this reader's shortcut through it. An over-count at the root
   stays the loss `page-not-held`, which is decided from the source's count before anything is
   written (ADR 2596); restating the output's count does not hide it.
3. **A node reached by two routes is held to one number** whichever route its stated count agrees
   with, so the fence of ADR 2456 for two different counts now covers sound nodes too.
4. **`optimize` and `redact` refuse such a source by name**, as they refuse a count that is no
   integer: their output is a closure copy that rewrites no node, so they cannot ensure what the NOTE
   asks of them.

## Cost

A document whose producer wrote a wrong integer count now converts with one more rewrite listed, and
is refused by `optimize` and `redact` where it was carried. The validator's page tree walk is paid
once more per examination, under the same depth and node bounds as the other page tree rows.
