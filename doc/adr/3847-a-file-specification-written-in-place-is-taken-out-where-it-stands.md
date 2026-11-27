# 3847 — A file specification written in place is taken out where it stands

Session 2286. Status: **accepted**. Replaces ADR 3834's refusal of a specification written inside
a kept object (`EMBEDDED_FILE_WRITTEN_IN_PLACE`) for two places. Context: ADR 1099, ADR 2536,
ADR 3834; ISO 19005-2 section 6.8, ISO 19005-4 section 6.9 (paraphrased); ISO 32000-2 §7.11.3,
§7.11.4, §12.5.6.15 Table 184, §14.13.2.

## Context

The embedded-file rows name a specification written in place by the object holding it. The
removal of an inadmissible file removed references to specification objects, so a holder the
conversion kept was refused. ADR 3834 argued that removing a direct `/FS` would remove the
annotation that requires it; the removal need not do that.

## Decision

- A named holder's in-place specifications are judged one by one with the row's own judgement
  (`pdf_archive::table::embedded_file_fails`, the per-specification half of
  `embedded_file_is_itself_pdfa`), and only the failing ones are taken out; their embedded file
  streams are what the rewrite knows them by, as they have no numbers.
- **In an `/AF` array**, anywhere in the holder, the specification leaves the array, as a
  referenced one does; an array left empty goes (§14.13.2: an absent `/AF` says the same).
- **As the holder's own `/FS`** (Table 184: required of a file attachment annotation), the
  specification stays and its `/EF` and `/RF` go. What is left is §7.11.3's specification of a
  file by name; the annotation, its appearance, place and text are kept, nothing is composed, and
  the embedded file is the loss the report names, as for a referenced one. No part of ISO 19005
  this converter targets asks a file specification to embed its file.
- A failing specification anywhere else inside the holder (an action's `/F`, a collection item,
  a media clip), or a holder where no specification fails on its own reading, still refuses by
  name: each such entry needs a reading of its own.

## Consequences

`archive::a_file_no_part_admits_under_a_specification_written_in_place_is_taken_out_where_it_stands`
and `archive::a_file_no_part_admits_in_place_in_an_af_array_leaves_the_array` convert to
conforming files, which veraPDF passes. A *referenced* specification a kept file attachment
annotation names still refuses (`EMBEDDED_FILE_STILL_REACHED`) outside the files-as-pages profile;
the same `/EF` removal would answer it, and is left for its own round.
