# 2432 — A direct descendant's absent `/CIDToGIDMap` is written where the font is written

Status: implemented. Session 1799, the archive slot of batch one hundred and twenty. Context: ADR
2413, which brought a font written directly inside an object into the font population and left
`Test-plusminus.pdf` refused by name; ADR 2246, the edition test this decision reuses;
`doc/pdf-a-mitigations.md`'s entry for `fonts/cid-to-gid-map-present`.

## The reading

ISO 19005-2 section 6.2.11.3.2 and ISO 19005-4 section 6.2.10.3.2 require an embedded Type 2
`CIDFont` to state the entry, as a stream or as `Identity`. ADR 2246 settled what an *absent* entry
says: where ISO 32000-1 describes the file — every part 2 target, and a part 4 one for a file
labelled below PDF 2.0 (ISO 32000-2 §I.2) — that edition's table gives the entry `Identity` as its
default, so writing `Identity` states what the file already said; ISO 32000-2's Table 115 gives no
default, so a file labelled 2.0 or unlabelled keeps its refusal.

ISO 32000-2 §9.7.6.1 lets a Type 0 font's `/DescendantFonts` hold its `CIDFont` directly. **Where
the dictionary is written changes nothing about what its absent entry said**, so the same
statement is owed under the same two conditions. `Test-plusminus.pdf` is that case: labelled
PDF 1.6, one Type 0 font object writing its embedded `CIDFontType2` inside the array.

## The premise that did not hold as worded

The brief put the condition as "where the font program's glyph order makes `/Identity` true". The
tree's remedy reads no program: ADR 2246 writes the edition's default, and deriving a map from the
embedded program stays a reading of the font this converter does not make. The direct case takes
the same argument and the same refusals, and nothing here opens the program.

## The decision

1. `pdf_archive::direct_cid_fonts_stating_no_cid_to_gid_map` lists, over the same population and
   the same selection as the row, every direct embedded Type 2 `CIDFont` stating no entry, each with
   `Some(type0)` where it is the first element of a `/DescendantFonts` array written directly in a
   Type 0 font that is an object of its own, and `None` for any other holder.
2. `sites::cid_to_gid_maps` accepts the row's findings at no object only when that list places
   every one of them — its length equals the count of the row's findings at no object, and no
   element is `None`. Otherwise the whole requirement is refused with
   `CID_TO_GID_MAP_IN_ANOTHER_OBJECT`, which now names the shapes still refused: a direct font in a
   holder other than its Type 0 font's array, and a direct font's entry present with neither shape.
3. The rewrite writes `/CIDToGIDMap /Identity` into the array's first element of each such Type 0
   font object, the array and the dictionary otherwise as they were (`CidToGidSites::descendant_in`).

## What it costs

A second walk of the font population for a document whose row reports a font at no object, and
nothing otherwise. `Test-plusminus.pdf` converts at 2b and 4 under both shipped profiles, and
veraPDF 1.30.3 passes all four outputs (`doc/history/1799`). This reader already draws an absent
entry as the identity (ADR 2246), so no glyph moves.
