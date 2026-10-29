# 2583 — A page tree Table 30 rules out is failed by the checker, at two rows of its own

Status: implemented, in `pdf-archive` (`table::file_structure`, rows
`file-structure/page-tree-root-states-no-parent` and `file-structure/page-tree-kids-name-dictionaries`;
`coverage`, section 5.1 of both parts) and `pdf-transform` (`archive::decision`, the rows' refusal).
Session 1874, the third archive slot of batch 128.
Context: ISO 32000-2 §7.7.3.2 Table 30 and §7.3.10; ISO 19005-2 and ISO 19005-4 section 5.1; ADR
2558 item 5 and its "what it frees" paragraph, which found the gap; ADR 2444 and ADR 2456, the
`/Kids` and `/Count` rows' narrowing.

## The gap

The converter declines a source whose page tree root states `/Parent`, which Table 30's `/Parent`
row does not permit in the root node, or whose `/Kids` holds an entry naming no dictionary (§7.3.10
makes a reference to an object the file does not hold the null object), and writes the tree this
reader recovered under `recovered-structure` (ADR 2558). `pdf-archive` passed both shapes:
`PDFBOX-6040-nodeloop.pdf`'s root names itself as its parent and lists a null kid, and veraPDF cannot
parse such a tree.

## Decision

1. **Two rows, not a widening of the `/Kids` row.** `page-tree-node-kids-is-an-array` is the
   entry's presence and type, and the root's `/Parent` is a different entry; a report naming each by
   its own row says which sentence of Table 30 failed. Both are section 5.1's, as the `/Kids` and
   `/Count` rows are.
2. **The root is the catalog's `/Pages`**, held to the row where it is a node by the reading the
   walk already takes (`/Kids` stated or `/Type /Pages`). Only the root is judged: the row requires
   `/Parent` everywhere else, and whether a non-root node's parent is the right one is not judged.
3. **A child is any entry that resolves to a dictionary.** The row's second sentence limits children
   to page objects and page tree nodes; narrowed to what a reader needs to open a child at all, the
   converter's own condition, so the checker and the converter decline the same trees. Whether a
   dictionary is of either kind, and whether it is written by reference, are not judged here.
4. **One finding per node**, naming the first such entry and how many follow. Each node's array is
   read once, as the bounded walk reads it (64 levels, 2^20 nodes), so nothing an input reaches is
   unbounded; a `/Kids` of two million nulls leaves the high-water where the `/Kids` row's own walk
   put it.
5. **The converter's answer is a refusal by name.** A source the converter's walk passes and the
   checker fails has the fault at a node written directly (inside the catalog or a parent's
   `/Kids`), which the walk does not follow; it is refused with a sentence saying so rather than by
   the table's default. Building the rebuild for that case is not this decision.

## What it moves

No converter decision for a node written as an object of its own: such a source is declined or
rebuilt before any row is decided, and the rebuilt root states neither fault. A remedy for a null
kid beyond `recovered-structure` is `doc/questions/A727`'s.

## Measured

- **Memory**: a `/Kids` of 2 000 000 nulls leaves the high-water `getrusage` reports at 80 748 kB
  after HEAD's walk and at 80 748 kB after both rows have judged it.
- **The census**, with and without the CJK pack, against HEAD's: no line moves by either row, as
  section "What it moves" predicts; PDFBOX-6040's node loop fails both rows at 2b and at part 4
  (`crates/pdf-archive/tests/witnesses.rs`).
