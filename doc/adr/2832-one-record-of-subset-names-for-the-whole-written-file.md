# 2832 — One record of subset names for the whole written file

Round 2007, the ledger lane.
Status: **accepted and built** in `pdf-model` (`variable_text.rs`: `SubsetNames`, `for_a_file_in`,
`stated_subset_names`, `is_subset_tagged`; `appearance.rs`: `for_annotation_in`, `for_saving`, `toggling_states`;
`view/writing.rs`: `Update`).
Extends ADR 2818 section on subset tags, which made the tags distinct within one resource dictionary only.
Clauses: ISO 32000-2 §9.9.2, §7.5.6.

## The reading

§9.9.2: "The tag shall consist of exactly six uppercase letters; the choice of letters is arbitrary, but different
subsets of the same font in the same PDF file shall have different tags." The writer digests a tag from the program
and the glyphs kept (ADR 1425), so two different subsets can share a tag, and the owner's audit found two glyph sets
of the bundled face that do. The unit the clause names is the *file*, not the resource dictionary: a save that writes
three widgets' appearances writes three dictionaries into one file.

An incremental update (§7.5.6) appends to the file and keeps every font it already held, so the file's own names are
in the same file too. Whether an earlier font under the same name *is* a different subset would need its program
compared, and the clause asks nothing of one subset under two tags; so a name the file states is passed over without
comparing contents, and only names this write itself chose are compared, so that one subset written into two
appearances keeps one name.

## The construction

- `SubsetNames` is one record per written file: each subset written, under its name, and the subset-tagged
  `/BaseFont` and `/FontName` names the document's cross-reference information names (direct values followed to a
  bounded depth). The document is read the first time a subset is named and never otherwise, so a save that writes
  no face from this machine costs nothing more (principle 2).
- A name taken by a different subset of this write, or stated by the file, moves to the next tag (`retag`, base
  twenty-six, unchanged).
- `view::writing::Update` holds one record for the save, and every appearance it writes — `for_saving` (whose
  regenerated, constructed and toggling arms are three of the six calls), added annotations and retyped FreeText —
  names through it.
- `appearance::for_annotation` keeps its signature and names against the document with a record of its own;
  `for_annotation_in` takes the caller's record. The archive conversion calls `for_annotation` twice, and adopting
  `for_annotation_in` there with one record for the converted file is the PDF/A lane's change, not this round's.
- `for_a_file(resources)` keeps the owner's audit test's signature and names within one dictionary.

## What is left

- The archive conversion's own writers (`archive/fonts.rs`'s embedding) and its two `for_annotation` calls do not
  yet share one record.
- Appearances imported whole from an FDF (`write_imported_appearances`) keep the names their producer gave them, and
  are not entered in the record before this save's own subsets are named.
