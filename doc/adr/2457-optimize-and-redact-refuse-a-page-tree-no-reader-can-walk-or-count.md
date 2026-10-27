# 2457 — `optimize` and `redact` refuse a page tree no reader can walk or count

Status: implemented. Session 1811, the second half of the archive slot of batch one hundred and
twenty-one. Context: ADR 2444's consequence, ADR 0852 (a writer's refusal is exit 4).

## The finding

ADR 2444 found that `optimize` writes `poppler-91414-0-53`'s root node, whose `/Count` names a
stream, into a file veraPDF cannot parse, and that it and `redact` asked only
`optimize::refuse_a_document_only_recovery_reads`. Both verbs carry the producer's page tree as
it stands through `crate::optimize`'s closure copy, so every node ISO 32000-2 §7.7.3.2's Table 30
makes unwalkable (`/Kids` missing or no array) or uncountable (`/Count` missing or no integer)
reaches their output unchanged.

## The decision

Both verbs ask `archive::page_tree::refuse_a_page_tree_no_reader_can_walk` after the recovery
refusal, the check ADR 2444 built, held to both entries; the module is `pub(crate)` for that. A
document it refuses ends `Refusal::Reconstructed`, exit 4, naming the object and the entry.

## Why not restate the count here as the conversion does (ADR 2456)

The restatement is an object rewrite, and these verbs copy objects: giving them one means an
`Assembly::replace` of the node with every reference inside it mapped into the output's
numbering, a second construction of what `archive::rewrite` already does. The cost of the refusal
is the documents that would have the restatement: in the corpora that is the two `91414` files
and `T02-02_008`, each of which converts to PDF/A. Taking the restatement into these verbs is
their own change, priced by those documents.
