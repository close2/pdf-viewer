# 2154 — A departure's neighbour that stops short says so: the merge's field walk and a save's template pages

Session 1659, the clause slot of batch one hundred and two, under the project owner's switch to code
improvements and bug fixes of 2026-10-14. Status: **accepted**. Clauses: ISO 32000-2 §12.7.4.1,
§12.7.4.2, §12.7.8.3.3, §12.8.1; both rows stay `departed`. Context: ADRs 1198, 1239, 1622, trap 5.

## 1. What the re-reading found

The twenty `departed` rows were read against their clauses and the code they name. Two of them sat
beside code that stopped short of the clause in silence, which is the failure trap 5 names and which
neither row's departure licenses:

- **§12.7.4.1.** "An interactive PDF processor shall not limit the range of inheritance for field
  dictionaries." `pdf-model` bounds the walk at `MAX_FIELD_ANCESTRY` (256) and reports reaching it
  (ADR 1198). `pdf-transform`'s `merge::walk_field` stopped at 64 and returned: a field deeper than
  that took no part in §12.7.4.2's collision test, and a signature field there kept the `/V` that
  `plan_unsigned` withholds from every other (§12.8.1), with no `Warning`.
- **§12.7.8.3.3.** Table 252's `/Pages` describes "new pages that shall be added to the target
  document". An import adds a template page to the view (`ViewState::appended_pages`), and
  `ViewState::save`'s §7.5.6 update writes no page into the page tree, so a save after such an
  import left the page out without a word.

## 2. The decision

**A walk or a writer beside a departure that stops short of the clause names what it left out, on
the channel its caller already reads.** The merge walks to 256, the number ADR 1198 measured, and
every hierarchy that runs past it is a `Warning` naming the field under which the walk stopped
(`a_field_hierarchy_past_the_merges_bound_is_named`, which fails with the warning removed). The test
sits after the `/T` check, so a terminal field at the bound whose widgets are its kids is not called
cut. `Written::unwritten_pages` counts the template pages a save did not write, and `viewer-core`'s
save reports them beside the password fields and the unappeared annotations
(`an_imported_template_adds_a_page_the_document_already_held`).

## 3. What this does not decide

Neither departure moves. Writing a template page into the page tree, which §12.7.8.3.3's `/Rename
true` then needs, is §7.5.6's construction for a later round; the merge's bound stays, because a
hostile `/Kids` can nest without end and principle 3's budgets outrank an unbounded walk.
