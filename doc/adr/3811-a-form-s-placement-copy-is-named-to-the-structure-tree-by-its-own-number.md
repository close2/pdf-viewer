# 3811 — A form's placement copy is named to the structure tree by its own number

Session 2277. Status: **accepted**. Context: ADR 1363 (a form met differently at two placements
is written as copies), ADR 3717, ADR 3725; ISO 32000-2 §8.10.1, §14.7.5.2, §12.5.6.23.

## Context

A form drawn more than once that a region meets differently is written as the producer's form
for a placement missed and a copy per distinct edit (ADR 1363). Two things followed the form's
number rather than the copy's:

1. The report of the stale entries each edited form leaves out (`/PieceInfo`, `/LastModified`,
   `/Metadata`, ADR 3717) said it once per stream written, so a form written twice was named
   twice for the same entries of the same object.
2. The structure tree names a form's sequence by a §14.7.5.2 marked-content reference whose
   `/Stm` is the form. The copy is a new object no reference names, so the part of the sequence
   the copy still shows was outside the tree, and a sequence reached at the copy counted as the
   form's own: an element was reported as having lost "every content item" while the producer's
   form still drew it at the placement missed.

## Decision

- The stale entries are reported once per form, with the number of streams it was written as.
- A sequence the walk reached inside a placement copy records the copy (`Reached::copy`, the
  page's form edit). The form's own content item is reached only by a sequence outside any copy;
  each copy is one content item more of the element. An element whose sequence a copy still
  shows part of is restated with a marked-content reference `<< /Type /MCR /Pg <page> /Stm
  <copy> /MCID n >>` added to its `/K`, the copy's number taken from the slot the writer gave it.
  A copy whose sequence the region took whole gains nothing: there is nothing left to name.
- `/StructParents` is carried on the copy as the producer wrote it on the form, so the parent
  tree reaches the same element from either stream.

## Consequences

`redact::a_form_written_as_two_streams_is_reported_once` and
`redact::a_placement_copy_is_named_to_the_structure_tree_by_its_own_number` hold both.
