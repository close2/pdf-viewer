# 2838 — The archive conversion names its subsets through one record

Round 2010, the PDF/A lane.
Status: **accepted and built** in `pdf-model` (`variable_text.rs`: `SubsetNames::name_program`) and
`pdf-transform` (`archive/fonts.rs`: `Substitutes::names`; `fonts/machine_cid.rs`, `fonts/rekeyed.rs`,
`fonts/pack.rs`; `prepare.rs` and `prepare/annotations_and_attachments.rs`).
Extends ADR 2832, whose "What is left" named this change as the PDF/A lane's.
Clauses: ISO 32000-2 §9.9.2.

## The reading

§9.9.2: "the choice of letters is arbitrary, but different subsets of the same font in the same PDF file shall
have different tags." ADR 2832 read the unit as the file and built one record for a save. An archive conversion
writes one file too, and it has more writers than a save: the embedding of a face from this machine
(`machine_cid`), the re-keying of an `Adobe-Identity` font onto a shipped face or the pack (`rekeyed`), the pack's
CID-keyed subset (`pack`), and the appearances `pdf_model::appearance` constructs for annotations and widgets that
have none. Each digests its tag from its own program and glyphs (ADR 1425), so two of them can arrive at one tag,
and every name the source states is in the converted file as well, since the conversion rewrites the whole file
and keeps its fonts.

## The construction

- `SubsetNames::name_program(document, name, program)` names a subset that a writer holds as a program rather
  than as a font dictionary built in `variable_text`. The program's bytes are the subset's identity: the same
  bytes under the same name keep it; any other subset under that name, or a name the document states, moves this
  one to the next tag (`retag`, unchanged). A name without the tag is returned as it is.
- `Substitutes` carries one record for the embedding pass. Each of the three subset writers names its subset
  through it the moment the subset exists, before the name reaches a `/BaseFont`, a `/FontName` or a widening
  record. A pass that is begun again around a photographed font starts a fresh `Substitutes`, so a name chosen
  by a pass that was thrown away takes no tag from the pass that is kept.
- `prepare` hands a copy of the embedding's record (an empty one where the embedding refused) to
  `prepare_appearances` and then `prepare_field_appearances`, which call `appearance::for_annotation_in` with it.
  The embedding is prepared first, so the appearances name against every subset it wrote.

Each choice is a renaming within §9.9.2's freedom ("the choice of letters is arbitrary"): the profile decides
nothing, no mark changes, and no loss is named, because a tag carries no content.

## Cost

The record keeps each program it named for the comparison, so a conversion holds its subsets once more while it
runs. The CJK pack's subsets are the largest; they are already held whole by the stream that writes them.

## What is left

- Appearances imported whole from an FDF keep their producer's names (ADR 2832), and the described route of
  `archive/fonts/described.rs` writes a face's whole program under its own PostScript name, which carries no
  tag and so is no subset's.
