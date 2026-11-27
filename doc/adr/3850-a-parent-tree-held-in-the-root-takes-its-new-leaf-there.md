# 3850 — A parent tree the structure tree root holds directly takes its new leaf there

Session 2287. Status: **accepted**. Supersedes the exception in ADR 3846's second decision (a
parent tree held directly in the structure tree root with `/Kids` leaving the copies the form's
key). Context: ADR 2671, ADR 3846; ISO 32000-2 §7.9.7 Table 37, §14.7.5.4.

## Context

ADR 3846 puts a copy's new parent-tree pairs in one new leaf after the root's `/Kids`, so that no
producer leaf and no `/Limits` above one is rewritten. A new leaf must be an object: Table 37 makes
`/Kids` "an array of indirect references to the immediate children of this node". Where the tree's
root is an object, the leaf's number is added to its `/Kids` once the writer gives it one. Where
the structure tree root writes the tree inside itself, there was no restated object to add the
number to, and the copies kept the form's key.

## Decision

The structure tree root is restated anyway (its `/ParentTreeNextKey` moves), so it is that object:
`traces::Restated::parent_tree_kids` carries the leaf's number, in the output's numbering, and
`restated_dict` adds it at the end of the `/Kids` of the `/ParentTree` the root states, after the
rest of the root is carried. The leaf states `/Limits` and its `/Nums`; the producer's leaves and
the root's other entries are as written. A root the catalog states directly (ADR 3845) takes it the
same way, through the catalog's restated root.

## Consequences

`redact::a_parent_tree_of_kids_held_in_the_root_takes_the_copy_s_entry_in_a_new_leaf`: the tree
keeps its producer leaf with `/Limits [0 1]` and gains a second kid, `/Limits [2 2]`, holding the
copy's entry. A tree past the walk's node or depth bound still keeps the copies on the form's key.
