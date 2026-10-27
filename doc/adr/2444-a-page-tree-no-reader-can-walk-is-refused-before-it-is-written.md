# 2444 — A page tree no reader can walk is refused before it is written

Status: implemented. Session 1805, the page-tree half of the archive slot of batch one hundred and
twenty. Context: `doc/todo/72` (its owed fuzzed documents), ADR 0947, ADR 0860,
`crates/pdf-transform/src/optimize.rs`'s `refuse_a_document_only_recovery_reads`.

## The reading

§7.7.3.2's Table 30 makes both entries of a page tree node required and states their types:
`/Kids` "( Required ) An array of indirect references to the immediate children of this node.",
`/Count` "( Required ) The number of leaf nodes (page objects) that are descendants of this node
within the page tree." This reader draws past a node that breaks either: `pdf_model::Pages` counts
the leaves where a `/Count` is missing or no number, and a node declaring `/Type /Pages` with no
`/Kids` array reaches no page. The converter carried such a node as it stood, so three pdf.js
documents converted at 2b and at part 4 into files veraPDF reports it cannot parse:
`poppler-91414-0-53` and `-54`, whose root's `/Count` names a stream, and `poppler-937-0-fuzzed`,
whose root's `/Kids` lost its bracket to a fuzzed byte and is a bare reference.

## The decision

`archive::page_tree::refuse_a_page_tree_no_reader_can_walk`, asked by the verb beside the recovery
refusal and before stage 1: every node the catalog's `/Pages` reaches is read, depth and node count
bounded as the reader bounds them, and a node whose `/Kids` is missing or not an array, or whose
`/Count` is missing or not an integer, refuses the document as `Refusal::Reconstructed` (exit 4)
with the clause, the object and which entry. A kid stating neither `/Kids` nor `/Type /Pages` is a
leaf, the reader's own reading, and is not held to a node's entries.

## Why not restate the count

The row's NOTE puts the restatement on the writer: "[a] PDF writer shall ensure that the value of
the Count key is consistent with the number of entries in the Kids array". So the two 91414
documents have a remedy whose result the standard states, and it is not built here, by ADR 0947's
second rule: nothing is changed that no failed requirement asked for, and no checked requirement
of ISO 19005 is failed by a node's types — each part's section 5.1 is
`conformance/adheres-to-the-base-standard`, `Unchecked`. The route that keeps the rule is a 5.1
row of its own beside `file-structure/hexadecimal-string-holds-only-digits`, failed by a `/Count`
that is no number and answered by the writer's restatement; its cost today is these two documents.
A node with no `/Kids` array is refused on any route, because the tree states no child and the
pages a reader finds without one are not the producer's.

## Consequences

- The three documents are refused by name at both targets under both shipped profiles and nothing
  is written; the census lists them with the verb's sentence instead of counting three outputs
  that do not open as conversions.
- `optimize` writes the same `/Count` stream for `poppler-91414-0-53` into a file veraPDF cannot
  parse; it and `redact` call only the recovery refusal, and giving them this one is their verbs'
  change to make.
