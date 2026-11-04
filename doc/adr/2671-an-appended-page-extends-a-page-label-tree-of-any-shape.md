# 2671 — An appended page extends a page-label tree of any shape

Session 1922, a single archive round.
Status: accepted and built in `pdf-transform` (`archive/preserve.rs`: `labels_extended`,
`LabelWalk`, the refusals `LABELS_NOT_A_TREE`, `LABELS_CYCLE`, `LABELS_OUT_OF_ORDER`,
`LABELS_PAST_THE_PAGES`, `LABELS_TOO_DEEP`).
Context: ADR 0954 (labels follow a changed page count), ADR 1025 section 4 (the appended pages'
empty range), ADR 1014 (the appended page), ADR 2669 (the Defence document refused because its page
"cannot be composed").

## What the refusal was, measured

ADR 2669 attributes the Defence document's refusal to "a number tree with `/Kids`". **It is not
one.** Its catalog states a flat `/PageLabels << /Nums [0 … 12 <</P (p. 38)>>] >>`, a range for each
of the thirteen pages its root counts, and it holds twelve: the range at key 12 is the page not
held. The appended page's index is 12, and the one refusal, `LABELS_NOT_EXTENDABLE`, fired both for
`/Kids` and for a key at or past the first appended page, with a sentence that named only `/Kids`.
Two remedies follow, and both are built.

## Decision

1. **A tree with `/Kids` takes one new leaf as its root's last kid**, `<< /Limits [n n] /Nums [n << >>] >>`,
   written as a new indirect object (Table 37: kids "Shall be an array of indirect references").
   Every key it holds is past every key before it, so no producer node's `/Limits` stops being true;
   Table 37 gives the root none, so the root's `/Kids`, one reference longer, is the only producer
   object rewritten whatever the depth. Appending to the right-most leaf would rewrite that leaf and
   every `/Limits` above it — more producer objects for the same tree. A root `/Limits`, which the
   table does not permit, is widened rather than left saying the tree ends earlier. This writes
   structure and no content: a number-tree node, and a label range whose dictionary is empty, so no
   mark and no word reaches any page (CLAUDE.md's authoring line).
2. **A flat tree's range at a page index the document does not hold is taken out.** §12.4.2 keys a
   range by page index, and an index at or past the first appended page is one only an appended
   page will have, so that range labels none of the pages carried; left in, it would number the
   appended page as the producer's, which ADR 1025 section 4 rules out. In a tree with `/Kids` such a
   range sits in a producer's leaf, and it stays refused (`LABELS_PAST_THE_PAGES`), with no document
   known to hold one.
3. **Refused by name, and only what cannot be read**, in one walk of every node before anything is
   written: a node or kid that is not a dictionary, a `/Kids` or `/Nums` that is not an array, a
   `/Nums` of odd length or a key that is not an integer, a root of both entries (§7.9.6: "either
   Kids or Names but not both") — `LABELS_NOT_A_TREE`; a node reached twice — `LABELS_CYCLE`; keys
   not ascending across the walk — `LABELS_OUT_OF_ORDER`; more than 64 levels, the bound
   `pdf_syntax::tree` reads to — `LABELS_TOO_DEEP`. The odd `/Nums` was not checked before, and a
   pair appended after a lone key would have paired the wrong way.

## What moves

The census command of ADR 2669, run after this change and set against its run after ADR 2669: the
Defence document converts under `state-every-loss` and `no-data-loss` at 2b and at part 4, and
nothing else changes state. Its four outputs pass veraPDF 1.30.3, and their trees end
`11 <</P (p. 37)>> 12 << >>`.

## Tests

`archive::preserve::tests`: a flat tree, the Defence shape, a two-level `/Kids` tree (the new leaf,
its `/Limits`, the root without one), and every refusal planted, the depth bound at both sides.
`tests/archive/recovered.rs`: a flat tree with a range at the page not held, a root over one leaf
and a root over an intermediate node over two leaves, each converted under `append-statement` at
both targets, checked conformant, and read back through `pdf_model::page_label::PageLabels`: the
twelve producer's labels unchanged, the appended page's the empty string. Withholding the new leaf
makes that test fail with the appended page labelled `A-3`.
