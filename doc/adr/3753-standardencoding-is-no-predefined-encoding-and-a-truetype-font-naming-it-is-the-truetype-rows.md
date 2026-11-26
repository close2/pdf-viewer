# 3753 — `StandardEncoding` is no predefined encoding, and a TrueType font naming it is the TrueType row's

Session 2263. Status: **accepted and built** in `pdf-font` (`glyph_names::PERMITTED_ENCODING_NAMES`,
`base_encoding`, `repair::ENCODING_NAME_STANDARD`), `pdf-model` (`type3::encoding`), `pdf-archive`
(`repairs::MAPPING`, `table::fonts::font_maps_read_as_written`) and `pdf-transform`
(`archive/font_maps.rs`, `archive/font_files/type1_names.rs`). Answers audit 2260's B01 by the
owner's `doc/questions/A1082` (option 1). Clauses: ISO 32000-2 Annex D Table D.1, §9.6.2.1
Table 109, §9.6.5.1 Table 112, §9.6.5.4; ISO 19005-2 section 6.2.11.6, ISO 19005-4 section 6.2.10.6.

## Decision

Table D.1: "PDF processors shall not have a predefined encoding named StandardEncoding." Table 109
lists three names, and the reader admitted a fourth. Now a simple font — Type 1, TrueType, Type 3 —
whose `/Encoding` or `/BaseEncoding` names `StandardEncoding` has named no encoding: the built-in
encoding stands, as for any name Table 109 does not list. `BaseEncoding::by_name` keeps the table,
because §9.6.5.1 and §9.6.5.4 fill undefined codes from it; it is the *name* a font may not use.

The repair is a class of its own, `font/encoding-name-standard`, beside `font/encoding-name-unknown`,
because what it breaches depends on the font. On a nonsymbolic TrueType font the name is the one
section 6.2.11.6 asks to be `MacRomanEncoding` or `WinAnsiEncoding`; that row reports it on its own,
and `standard-truetype-encoding`'s rewrite restates it with a code-by-code proof. Restating it as
read instead (`font-maps-read-as-written`, ADR 3570) removes the name, which moves the font onto
§9.6.5.4's route for a font with no `/Encoding`, and the proof refuses: two archive tests that
convert mechanically would have needed a loss. On every other font the name is a map restated as
read, and the removal is exact.

**The split is made where the font is known, and that is the row, not the mapping.** `MAPPING` maps
a class by its name alone and the reader places an encoding repair at the `/Encoding` object or the
font descriptor, not the font. So the class maps to both rows, and
`fonts/font-maps-read-as-written` passes over a repair of it placed where only nonsymbolic TrueType
fonts naming the standard encoding are; `font_maps` in the converter passes over the same fonts by
the same test. Two classes chosen inside `pdf-font` would have put an ISO 19005 distinction into the
reader, which knows nothing of archival parts, and a per-font variant of `Breaches` would have made
every caller of `breaches_row` answer a question only this row asks.

The converter's Type 1 rewrite of a CFF-flavoured TrueType program (`type1_names`, ADR 3078) wrote
`/BaseEncoding /StandardEncoding`, so that a code the producer left unmapped reached nothing. It now
tries four bases in order — none (the program's built-in encoding, Table 112), `WinAnsiEncoding`,
`MacRomanEncoding`, `MacExpertEncoding` — and writes the first under which the code-by-code
comparison holds. The order is a choice: none is first because it states the least, and the named
three follow in Table 109's terms. The archive census found the reason: `issue215.pdf`'s program
fills with its own encoding codes the producer left unmapped, so no base refused it, and
`WinAnsiEncoding` holds.

## Consequences

`examples/standard_encoding_census.rs` finds four corpus documents naming the encoding and no page
drawn differently. A Type 3 font naming it is read as naming nothing and noted; the converter does
not restate a Type 3 font's maps, so such a file is refused by the output's own section 5.1 check,
by name. An unknown name in a Type 3 font is still read as nothing without a note.
