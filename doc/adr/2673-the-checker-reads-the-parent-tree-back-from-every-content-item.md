# 2673 — The archive checker reads the parent tree back from every content item the structure tree claims

Session 1924, a single archive round.
Status: accepted and built in `pdf-archive` (`table/file_structure/parent_tree.rs`:
`content_items_find_their_elements`, the row
`logical-structure/content-items-find-their-elements-through-the-parent-tree`) and answered in
`pdf-transform`'s decision table (`A_PARENT_TREE_THAT_LOSES_A_CONTENT_ITEM`, not built yet).
Context: ADR 1163 (the appended page described in the structure tree), ADR 2672 (a parent tree of
any depth extended), ADR 2669 (the census command). Clauses: ISO 32000-2 §14.7.5.4, Table 359,
Table 354's `/ParentTree`, §7.9.7; ISO 19005-2 and ISO 19005-4 section 5.1.

## What was missing, measured

Round 1923 withheld the parent tree's new leaf, so no appended page's marked content could be
found, and `pdf_archive::check` still answered `Conforms` at 2a and part 4. No row of the table
read `/StructParents` at all: the structure rows were `MarkInfo`, the root's presence, the role map
and `/Lang`.

## Where the clause is

ISO 19005-2 states no parent tree requirement of its own: 6.7.2.1 binds Level A to ISO 32000-1's
§14.8 and 6.7.3.3 roots the hierarchy "as described in" §14.7, both Level A only. ISO 19005-4's 6.8
states nothing normative. The sentence is the base standard's (§14.7.5.4; ISO 32000-1 §14.7.4.4 in
the same words), binding every file that has a structure tree, so the row is cited at **section 5.1
of both parts** and binds every target (`Applies::Always`), beside the page-tree rows in the
`file_structure` tranche, which is where this checker keeps the base standard's rows.

## Decision

1. **The population is read from the structure tree, not from content streams.** §14.7.5.1.1 makes
   a content item something associated with an element, and §14.7.5.2 associates a sequence by an
   element's `/K` naming it. The brief asked for every page whose content holds an `/MCID`; an
   `/MCID` no element claims is not a content item, and failing it would fire on a condition the
   clause does not state (trap 11). Each claimed sequence's holder — `/Stm` where stated, else the
   page — states `/StructParents`, the tree holds that key, the value is an array, and the entry at
   the `/MCID` names an element that claims the sequence; each claimed object states
   `/StructParent` and its entry names a claimant. One finding per holder.
2. **Two leniencies**: a sequence two elements claim is accepted where the entry names either, and
   an item with neither `/Pg` nor `/Stm` is passed over (Table 355's `/Pg` is its own requirement).
   `/Limits` are not consulted: whether the tree *contains* the entry is this clause's question,
   whether a descent by `/Limits` reaches it is §7.9.7's.
3. **Bounds, refused by name**: 64 levels and 2^20 elements and items for the structure walk, 64
   levels and 2^20 nodes and pairs for the parent tree; a parent tree that is not a number tree (a
   `/Nums` that is not an array, a key with no value or not an integer, a node reached twice, a kid
   not a dictionary) fails the row with the sentence naming which.
4. **The converter refuses a source the row fails**, `Because::NotBuiltYet`: rebuilding the entries
   is the structure tree read backwards and invents nothing, but it is not built in this round.

## What moves

`tools/bounded.sh --lock --tree 6 --build '--profile gates -p pdf-sandbox -p pdf-transform --bins'
-- cargo test --profile gates -p pdf-transform --test archive_corpus
the_stating_profiles_list_what_real_producers_files_refuse_on -- --ignored --nocapture --exact`,
against round 1923's after-run: the row refuses four pdf.js sources under both stating profiles at
2b and part 4 — `issue15340` and `issue20516` (a structure tree root with no `/ParentTree` though
its elements hold content items), `pdfjs_wikipedia` and `prefilled_f1040` (`Link` and widget
annotations an `OBJR` claims, stating no `/StructParent`, `mutool show` confirming). The first three
converted before, `prefilled_f1040` was refused already; no corpora source fails the row. Every
output both runs wrote is byte-identical once dates, `/ID` and UUIDs are masked, so no output changed.

**The remedy, decided because the count is small**: the converter is to *rebuild* the missing
entries — a `/StructParent` or `/StructParents` key from `/ParentTreeNextKey` on the holder, an
entry naming the claiming element, a `/ParentTree` written where the root states none — rather than
state a loss, because every value is the structure tree read in the other direction and invents
nothing a reader sees. It is owed to a later round; until then the row's answer is `NotBuiltYet`.
