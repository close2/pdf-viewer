# 2672 — A parent tree of any depth takes the appended pages' entries, by the page-label tree's construction

Session 1923, a single archive round.
Status: accepted and built in `pdf-transform` (`archive/preserve.rs`: `NumberTree`, `TreeRefusals`,
`DEEPEST_NUMBER_TREE`; `archive/tagged.rs`: `read_the_parent_tree`, the refusals
`PARENT_TREE_NOT_A_TREE`, `PARENT_TREE_CYCLE`, `PARENT_TREE_OUT_OF_ORDER`, `PARENT_TREE_TOO_DEEP`).
Context: ADR 1163 (the appended page described in the structure tree), ADR 2671 (the page-label
tree extended by a new leaf), ADR 2669 (the census command). Clauses: ISO 32000-2 §7.9.7 (Table
37), §14.7.5.4.

## What the refusal was, measured

`PARENT_TREE_NOT_EXTENDABLE` refused three shapes under one sentence: a `/ParentTree` whose root
holds `/Kids`, a root `/Nums` stated through a reference, and a `/ParentTree` that is not a
dictionary. Round 1922's census names eight corpus documents refused by it under the stating
profiles (`316267`, `459011`, `064034`, `219789`, `102952`, `UMTRI-2008-80`,
`LLMImplicationsforScience`, `me450f12project10_report`); `mutool show … StructTreeRoot/ParentTree`
shows each a root of `/Kids` alone, in an object of its own — the brief's premise held.

## Decision

1. **One number-tree reader and extender, both trees using it.** `NumberTree::read` walks every node
   once — ADR 2671's `LabelWalk`, moved and given its refusal sentences by the caller
   (`TreeRefusals`) — and `NumberTree::extended` adds pairs after the greatest key: a root holding
   `/Nums` takes them at the end of the array; a root holding `/Kids` takes one new indirect leaf
   as its last kid, `/Limits [least greatest]` of the pairs it holds, so the root is the only
   producer object rewritten at any depth and no producer `/Limits` stops being true (ADR 2671's
   argument, which is §7.9.7's and not the label tree's). The labels' own rule — a flat range at a
   page not held is taken out, one in a leaf refused — stays in `labels_extended`, since it is
   §12.4.2's and not §7.9.7's.
2. **The parent tree's keys come from §14.7.5.4's `/ParentTreeNextKey`**, corrected past the
   greatest key the walk read where the producer's value is not "greater than any that is currently
   in use", so an appended page's key cannot collide with one the tree holds at any depth; the new
   keys ascend from there, which `extended` checks rather than assumes.
3. **A root `/Nums` stated through a reference is written into the root dictionary**, which is
   rewritten in every case, rather than back into the array's own object: one site instead of two,
   and an array some other object might also name is not changed under it. The array object is
   left unreferenced, which no part of ISO 19005 forbids.
4. **The rewrite puts back only `/Kids`, `/Nums` and `/Limits`** into a parent tree root that is an
   object of its own, so any other entry the producer wrote there stays.
5. **Refused by name, and only what cannot be read**: a node or kid that is not a dictionary, a
   `/Kids` or `/Nums` that is not an array, an odd `/Nums`, a key that is not an integer, a root of
   both entries (`PARENT_TREE_NOT_A_TREE`); a node reached twice (`PARENT_TREE_CYCLE`); keys not
   ascending, now checked in a flat tree as well (`PARENT_TREE_OUT_OF_ORDER`); more than 64 levels
   (`PARENT_TREE_TOO_DEEP`). Each says why an appended page cannot be made findable.

This writes structure and no content: a number-tree node whose values are §14.7.5.4's arrays of
references to elements ADR 1163 already writes, so no mark and no word reaches a page.

## What moves

ADR 2669's census command, run after this change and set against round 1922's run of the
same command after ADR 2671: no document is refused by the parent tree any
more, and none is newly refused. Under `state-every-loss` `064034`, `316267`, `459011` and
`UMTRI-2008-80` convert at 2b and `064034` at part 4, and the five outputs pass veraPDF 1.30.3. The
rest — `102952` and `LLMImplicationsforScience` under `state-every-loss`, and all seven it
refused under `no-data-loss` (`316267`, `459011`, `219789`, `102952`, `UMTRI-2008-80`,
`LLMImplicationsforScience`, `me450f12project10_report`) — stay refused by other rows — a font the profile does not authorise to be
substituted, an extension schema described nowhere, overprint mode 1 — which the parent tree's
refusal had stood beside. `UMTRI-2008-80`'s output shows the construction: the parent tree's root,
two producer kids, gains a third, `/Limits [272 277]`, the six appended pages' keys from the
producer's `/ParentTreeNextKey 272`.

## Tests

`archive::tagged::tests`: a two-level `/Kids` parent tree whose `/ParentTreeNextKey` understates
its greatest key (the pages take 8 and 9, the new leaf `/Limits [8 9]`, the rewrite keeping the
node's other entries); a root `/Nums` through a reference written into the root; every refusal
planted, the depth bound at both sides. `tests/archive/statement.rs`:
`a_parent_tree_with_kids_gains_the_statement_pages_entries_at_any_depth` converts a root over one
leaf, a root over an intermediate node over two leaves, and a root `/Nums` by reference under
`append-statement` at 2a and part 4, and reads every page back through
`pdf_model::structure::ParentTree`: the producer's page and each appended page find a `P`. Withholding
the new leaf fails it at the appended page — while this tree's validator still passed that output,
so the read-back is the check, not the verdict.
