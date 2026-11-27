# 3846 — A form's placement copy is given a parent-tree key and entry of its own

Session 2286. Status: **accepted**. Supersedes ADR 3811's last decision (the copy carrying the
form's `/StructParents`). Context: ADR 1363, ADR 3811; ISO 32000-2 §8.10.1 Table 93, §14.7.5.2,
§14.7.5.4 Table 359, §7.9.7 Table 37, §12.5.6.23.

## Context

A form drawn more than once that a region meets differently is written as the producer's form and
a copy per distinct edit (ADR 1363). ADR 3811 carried the form's `/StructParents` onto each copy, so
form and copies shared one parent-tree entry: a sequence the region took whole from one placement
was still found, through the copy's key, as its element's content, and the tree could not tell the
streams apart. §14.7.5.4 asks an entry per content stream holding marked-content sequences, and its
key from `/ParentTreeNextKey`.

## Decision

- Each placement copy of a form that states `/StructParents` takes a key of its own, from the
  root's `/ParentTreeNextKey` or past the greatest key the tree holds where that is larger (a
  stated value that is not "greater than any that is currently in use" is not used). Its entry is
  the form's array with `null` at each `/MCID` the region took whole from that copy: every
  reference in it is the producer's. `/ParentTreeNextKey` is restated past the new keys.
- The pairs go at the end of the parent tree root's own `/Nums`, or into one new leaf with
  `/Limits` after the root's `/Kids` (Table 37 asks `/Limits` of leaves, not of the root), so no
  producer leaf is rewritten (the construction of ADR 2671). A parent tree held directly in the
  structure tree root with `/Kids` has no object for a new leaf's reference to be added to after
  numbering, and a tree past 100 000 nodes or the depth bound is not read: there the copies keep
  the form's key, as before.
- So that the tree and the parent tree say the same thing, an element marking a sequence of the
  form is given a `/Stm` reference to every copy that still holds the sequence, reached by the
  region or not (ADR 3811 added it only where reached in part).
- The form's own key and entry are untouched: the placement the region missed still finds its
  elements by them.

## Consequences

`redact::a_placement_copy_is_given_a_parent_tree_entry_of_its_own` holds a form of two sequences
drawn twice, one sequence taken from one placement: the form keeps key 1 and its entry, the copy
takes key 2 with `[null <kept element>]`, `/ParentTreeNextKey` is 3, and the kept element names
both streams. A shared form copied whole for one page (ADR 1196) still carries the form's key; it
is the same question one level up, left for a round of its own.
