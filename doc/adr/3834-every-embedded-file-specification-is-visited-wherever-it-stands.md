# 3834 — Every embedded file's specification is visited wherever it stands

Session 2283. Status: **accepted**. Context: review 2282 findings B02 and D01; ISO 19005-2 section 6.8,
ISO 19005-4 section 6.9 (paraphrased); ISO 32000-2 §7.11.3, §12.5.6.15 Table 184, §12.6.4.4,
§14.13.1.

## The finding

`for_each_embedded_file_specification` visited the cross-referenced objects carrying `/EF` and
the `/EmbeddedFiles` tree's direct leaves. A file attachment annotation may hold its `/FS` as a
direct dictionary (Table 184 asks for a file specification, direct or indirect), and so may a
`GoToE` action's `/F`, an `/AF` array and a collection item. Such a specification was read by no
row: a text file embedded under a direct `/FS` with no `UF` passed both part 2 rows, and the
converter reported its output as conforming.

## What changed

1. **The walk is every dictionary carrying `/EF` that the cross-referenced objects state**,
   an object itself or a dictionary written inside one, references not followed and the descent
   bounded. A specification written in place is reported at the object it is written in: it has
   no number of its own, and a converter reading the place finds no `/EF` there.
2. **Part 4's associated-file row** keeps its population: a specification written in place counts
   as associated where it stands in an `/AF` array (`Standing::InPlace { associated }`), which is
   what §14.13.1 makes an associated file; one with an object number is associated where an `/AF`
   array references it, as before.
3. **The converter**: `complete_file_specification` completes `/F`, `/UF` and `/AFRelationship`
   in every specification written inside an object it rewrites, so the mechanical rows reach the
   direct ones. The removal of a file a target does not admit removes references to specification
   objects; a place naming an object that carries no `/EF` is a holder, and the removal refuses by
   name (`EMBEDDED_FILE_WRITTEN_IN_PLACE`) unless section 6.3.1's removal already takes that
   annotation off its page. The refusal replaces the guess the old message described; building the
   removal of a direct `/FS` would remove the annotation that requires it, which is a different
   loss over the marks it draws, and it is not this one.
4. **D01**: `optional-content/order-lists-every-group` names a group `object N G`, as every other
   row names an object, rather than Rust's `Debug` form.
