# 3849 — A page's copy of a shared form is given a parent-tree entry of its own

Session 2287. Status: **accepted**. Extends ADR 3846 from a placement's copy to a page's copy of a
form it does not own (ADR 1196), and amends ADR 1196 §2's count of who owns a form. Context: ADR
1196, ADR 1363, ADR 3811, ADR 3846; ISO 32000-2 §8.10.1 Table 93, §14.7.5.2 Table 357, §14.7.5.4
Table 359, §12.5.6.23.

## Context

A form a redacted page draws but does not own is written as a copy for that page, and the
producer's form stays for every other placement (ADR 1196). The copy carried the form's
`/StructParents`, so a sequence the region took whole on one page was still found, through the
copy's key, as its element's content, and the walk read the form's own marked-content references
as reached: an element whose only content was the form's sequence lost it although the other page
still drew it. A redaction on one page changed another page's structure.

Writing the case down found a second defect under it. The ownership count read every reference
in the file, and a tagged form is named by one `/Stm` per sequence (§14.7.5.2) besides its `Do`:
every tagged form read as shared, was copied for its page, and the producer's stream — the covered
marks included — was carried into the output by the structure tree's references to it, though
nothing drew it any more. `a_tagged_form_one_page_draws_is_edited_in_its_own_place` held that.

## Decision

- **A `/Stm` is not a placement.** `reference_counts` does not count a marked-content reference's
  `/Stm`: it names the stream a sequence is in and draws nothing, so it cannot be a placement a
  replacement would destroy. A tagged form one page draws is the page's, and is edited in place.
- **A page's copy is a copy for the structure tree as a placement's is.** The sequences the walk
  reached in it are recorded as the copy's (`Reached::copy`), so the form's own references are
  not reached; the copy takes a key and an entry of its own from `/ParentTreeNextKey` with a null
  where the region took the sequence whole (ADR 3846's construction), and an element marking the
  form's sequence names the copy wherever the copy still holds it.
- A copy is matched to an element by its stream and `/MCID`, whatever page the element's
  reference states, and counted once per element: a form two pages draw is named by one reference
  whose `/Pg` can name only one of them, and the copy's own parent-tree entry names the element
  either way.

## Consequences

`redact::a_shared_form_copied_for_one_page_is_given_a_parent_tree_entry_of_its_own`: two pages
draw one form, one is redacted; the form keeps key 2, its entry and both elements, the copy takes
key 3 with `[null <kept>]`, and the kept element — whose `/Pg` names the other page — names both
streams. An element partly reached still has its description restated (ADR 3811), though the
other page still shows the sequence: the description is of content one page no longer holds.
A form shared through an ancestor's or another page's resources is still carried as the producer
wrote it wherever those resources reach it (ADR 1196), whether or not anything draws it there.
