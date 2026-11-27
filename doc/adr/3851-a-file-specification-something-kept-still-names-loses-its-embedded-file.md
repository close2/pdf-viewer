# 3851 — A file specification something kept still names loses its embedded file

Session 2287. Status: **accepted**. Replaces the refusal `EMBEDDED_FILE_STILL_REACHED` and extends
ADR 3847 to every place a specification is written in place. Context: ADR 1099, ADR 2536, ADR
2537, ADR 3834, ADR 3847; ISO 19005-2 section 6.8, ISO 19005-4 section 6.9 (paraphrased); ISO
32000-2 §7.9.6, §7.11.3, §7.11.4 Table 43, §12.5.6.15 Table 184, §12.6.4.4, §13.2.4.2.

## Context

`discard` of an embedded file no part admits removed every reference to its specification. A
specification object a kept file attachment annotation names could not be removed that way — Table
184 requires `/FS` — and outside the files-as-pages profile the document was refused. ADR 3847 had
answered the same case for a specification written in the annotation itself: it stays, naming its
file, without `/EF` and `/RF`. Written in place anywhere else — an action's `/F`, a media clip's
data, a name tree's leaf — the specification was still refused.

## Decision

- **A specification is a name with an optional embedded copy.** §7.11.3 specifies a file by name;
  §7.11.4's `/EF` (and `/RF`, which Table 43 asks only beside it) is what embeds it. Where the
  object naming the specification is kept, the specification is kept naming its file and the two
  entries go: what remains is every entry the producer wrote save the embedded file, which is the
  loss the report names. This holds for a specification object (`removed.at`, at whatever object
  the rewrite is handed it) and for one written in place, at any depth of its holder.
- **A list loses the entry.** In an `/AF` array the specification leaves the array (ADR 3847);
  in a name tree leaf's `/Names` the pair leaves, name with specification, as a referenced one's
  does: a list of files that names a file it no longer holds would list nothing true.
- An action's `/F` and a media clip's `/D` are the same specification under another entry.
  §12.6.4.4's embedded go-to action then names, by `/F`, a target document it does not embed;
  following it is the reader's search for that file, as for the annotation ADR 3847 kept. ISO
  19005-2's forbidden actions and annotation types are removed by their own rows whatever this
  leaves in them.
- Still refused by name (`EMBEDDED_FILE_WRITTEN_IN_PLACE`): a finding that names no object, and a
  named holder in which no specification fails on its own reading — nothing says which to take out.

## Consequences

`archive::a_file_no_part_admits_named_by_a_kept_file_attachment_loses_its_embedded_file`,
`..._in_place_under_an_action_loses_its_embedded_file_where_it_stands`,
`..._in_place_in_a_name_tree_leaf_leaves_the_tree_with_its_name`, and
`attachments_as_pages::an_annotation_naming_a_file_keeps_naming_it_without_the_file_where_no_page_answers_the_row`
convert to files our validator and veraPDF pass at 2b. In the files-as-pages profile a kept
attachment annotation still becomes a link to its pages (ADR 2537).
