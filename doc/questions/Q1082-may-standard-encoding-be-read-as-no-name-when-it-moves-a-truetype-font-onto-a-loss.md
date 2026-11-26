# Q1082 — `StandardEncoding` read as no name moves a TrueType font onto a loss in the archive: which way?

Source: round 2261, answering audit 2260's B01 (`doc/reviews/2260-specification-audit-annexes-d-e-f-i.md`).

## What the tree does now

ISO 32000-2 Annex D, Table D.1: "PDF processors shall not have a predefined encoding named
StandardEncoding. However, it is necessary to describe this encoding, since a font's built -in
encoding can be used as the base encoding from which differences may be specified in an encoding
dictionary." Table 109's `/Encoding` and Table 112's `/BaseEncoding` list only `MacRomanEncoding`,
`MacExpertEncoding` and `WinAnsiEncoding`. `pdf-font` admits a fourth name
(`glyph_names::PERMITTED_ENCODING_NAMES`) and reads a font naming `StandardEncoding` through Table
D.2's STD column. Every other unlisted name is read as naming nothing, with the repair
`font/encoding-name-unknown` noted, so an embedded Type 1 program's own built-in encoding stands.
The audit's test `standard_encoding_is_no_predefined_encoding_spec_audit` fails on that.

## The census

`crates/pdf-font/examples/standard_encoding_census.rs -- doc` (117 860 files, 117 833 opened) loads
each simple font that names `StandardEncoding` with the STD column spelled out as `/Differences`,
with the name removed, and as written. **4 documents hold 4 such fonts. No page would draw
differently**: one font's glyphs move, `doc/pdf.js/test/pdfs/issue20232.pdf` object 29
(`HPDFAA+Symbol_A`, TrueType, via `/BaseEncoding`), at codes 169 and 180, and its page shows that
font only at code 71.

## Why it is a question and not the change

Reading the name as absent, the change audit 2260 asks for, was built and tested in round 2261. In
the viewer it moved no page. **In the archive conversion it moved TrueType fonts**: the repair
`font/encoding-name-unknown` maps to ISO 19005-2 section 5.1's `fonts/font-maps-read-as-written`,
whose remedy removes the name and proves the font draws alike. For a nonsymbolic TrueType font that
proof fails, because §9.6.5.4 states a table only for the two listed names and for an encoding
dictionary, and an unlisted name and no entry are not read alike. So a source that converts today
by the mechanical rewrite `standard-truetype-encoding` (the name written as `WinAnsiEncoding` after
its proof) would be refused, or would need the loss `font-maps-as-read` authorised for a font whose
drawing nothing changed. Two existing tests show it: `archive.rs`'s
`a_non_symbolic_truetype_font_is_given_a_name_the_part_admits` and `truetype_cmap.rs`'s
`a_code_whose_standard_name_would_move_it_is_restated_where_the_narrow_rewrite_refuses`. The change
was reverted, and the departure is stated in `PERMITTED_ENCODING_NAMES`' comment and in D.1's
ledger row.

## The ask

1. **Read the name as absent for every simple font**, and give the archive's TrueType route its
   own answer: a `StandardEncoding` name on a TrueType font is left to `standard-truetype-encoding`'s
   rewrite and proof rather than to `font-maps-read-as-written` (a new repair identifier for the
   name, mapped to the TrueType row). About a round; Table D.1 met everywhere.
2. **Read it as absent for Type 1 fonts only** (`Type1`, `MMType1`), where Table D.1's reason, a
   program's built-in encoding, applies; TrueType keeps the STD column, recorded as a departure
   (§9.6.5.4 is TrueType's own subclause and names no table for any other name).
3. **Keep the name as it is read now**, recorded as a departure from Table D.1 (`departed`), with the
   census as its cost: no page draws differently either way.

The round recommends 1.
