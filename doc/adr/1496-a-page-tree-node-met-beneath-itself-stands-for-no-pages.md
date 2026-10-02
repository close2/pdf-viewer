# 1496 — A page-tree node met again beneath itself stands for no pages

Session 1330. Status: accepted and **built**.
Context: ISO 32000-2 §7.7.3.2 (Table 30), CLAUDE.md principle 3's time budgets, `doc/todo/10`'s fifth
member (the step and the membership test). Code: `crates/pdf-model/src/page.rs` (`Walk`, and the six
walks that take it: `collect_named`, `locate`, `collect`, `count_leaves`, `reaches_a_page`,
`find_leaf`). Test: `crates/pdf-model/tests/page_tree_nodes.rs`
(`nodes_that_name_their_own_ancestors_stand_for_no_pages`). Found by the `vfs_write` fuzz target
(ADR 1495). Amends the module's earlier reading that a `/Kids` cycle is the visit bound's to end.

## The defect

The six walks of the page tree were bounded by depth (`MAX_TREE_DEPTH`, 64) and by nodes visited
(`MAX_NODES_VISITED`, 2^20) and by nothing else, and the module named a `/Kids` cycle as what the
second was for. Both bounds hold, and neither is the right answer to a cycle. A node naming itself among its
kids, beside a second node naming the first and itself, branches twice at every level until one of
the bounds stops it: six objects counted **449 367 pages**, and every lookup among them spent its
million visits. `pdf-vfs` lists `pages/` by looking each one up, so a mount over a two-kilobyte file
did not answer in five minutes; the viewer's page count and every page turn paid the same.

## The reading

Table 30's `/Kids` is "[a]n array of indirect references to the immediate children of this node",
and each child's `/Parent` names that node: the structure is a tree, and a node that is its own
ancestor is not a child of anything — it is a cycle, and it stands for no pages. So each walk carries
the identities of the nodes above it (`Walk::path`, at most `MAX_TREE_DEPTH` long, so asking it is a
constant) and steps over a kid that is the node itself or one of its ancestors
(`Walk::closes_a_cycle`). The fixture's tree is three pages, each once, and the walk is linear.

**A node named by two different parents is not a cycle and is still walked twice.** That shape — a
DAG — is no tree either, but a page reachable two ways is a page the producer stated twice, and
whether to show it twice is a question about page numbering this ADR does not reopen; its cost is
what `MAX_NODES_VISITED` still bounds. `collect` starts at the root without its identity (the
entry it records says why), so its `Walk` is told the root's identity apart (`Walk::rooted`); the
six walks therefore agree on which kids they step over, as they agreed on everything else.

`MAX_NODES_VISITED` and `MAX_TREE_DEPTH` are unchanged: they bound a DAG and a deep chain, which the
path cannot see.
