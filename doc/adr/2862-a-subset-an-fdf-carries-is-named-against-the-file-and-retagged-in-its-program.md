# 2862 — A subset an FDF carries is named against the file, and retagged in its program too

Round 2023, the ledger lane.
Status: **accepted and built** in `pdf-font` (`subset_tag.rs`: `retagged`, `ProgramFormat`) and `pdf-model`
(`variable_text_imported.rs`: `SubsetNames::name_imported`; `variable_text.rs`: `SubsetNames::name`, `retag`,
`tagged_names`, `retag_programs`; `view/writing.rs`: `write_imported_appearances`, `write_added_appearance`).
Extends ADR 2832, which named every subset a save constructs; ADR 2838 did the same for the archive conversion.
Clauses: ISO 32000-2 §9.9.2, §12.7.8.3.2, §12.7.8.3.4, Table 117, Table 119, §9.6.3.

## The reading

§9.9.2: "different subsets of the same font in the same PDF file shall have different tags." An `/AP` imported
from an FDF (§12.7.8.3.2's field, §12.7.8.3.4's annotation) is written into the file whole, and its fonts are
subsets the FDF's producer named. Nothing made those names distinct from the target file's or from the ones
this save constructs, so the file the save wrote could hold two subsets under one tag. Retagging an imported
subset invents no mark: a tag is a name, and the stream's glyphs stay the producer's (ADR 1223).

Where the name lives in the program, the program moves too. Table 117 says a Type 0 `CIDFont`'s `/BaseFont`
"shall be the value of the CIDFontName entry in the CIDFont program", which is a compact font's Name INDEX.
A Type 1 program states `/FontName` in its clear text. §9.6.3 derives a `TrueType` `/BaseFont` from the `name`
table, where a producer may have written the tag as well.

## The construction

- `SubsetNames::name_imported` walks an imported appearance's direct objects, to a bounded depth. Each font held
  directly in a `/Font` resource dictionary is named through the save's one record (ADR 2832): the appearance
  stream's, a form's, and a Type 3 font's glyph procedures'. A font held by reference is the target's own and
  is among the names the file already states. The writer calls it for a field's imported `/AP` and for an
  imported annotation's `/AP` before their streams become objects.
- A subset is known by **every** tagged name it states: the Type 0 font's `/BaseFont`, each `CIDFont`'s, and each
  descriptor's `/FontName`. Table 119 lets the Type 0 name carry a `CMap` suffix. Before this, a Type 0 font over a
  compact `CIDFont` was compared by `TAG+Face-Identity-H` alone, so it could not meet a `CIDFont` named
  `TAG+Face`. And `retag` wrote that suffixed name onto the descendant and the descriptor. Both are corrected
  here: each name keeps what follows its plus sign.
- `retag` moves the six letters of every tagged name, and each program in a descriptor goes through
  `pdf_font::subset_tag::retagged`. That writes the new letters over the old wherever the format states the name
  (a compact font's Name INDEX, a Type 1 clear-text `/FontName`, an sfnt's `name` records in either encoding,
  and an OpenType `CFF ` table's Name INDEX). The six letters replace six letters, so no offset or length moves.
  Only an sfnt's checksums are restated. A program that cannot be decoded, or that never states the tag, is
  kept as it is. A restated program is written back under `FlateDecode` alone.
- The loop that moves a subset from tag to tag is bounded by the names already taken.

## What is left

Nothing of §9.9.2 that this tree writes: a save names what it constructs and what it imports, and the archive
conversion names what it embeds and constructs (ADR 2838).
