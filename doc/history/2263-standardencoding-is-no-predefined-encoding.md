# 2263 — `StandardEncoding` is no predefined encoding

Lane 3. Builds the owner's A1082 (option 1) for audit 2260's B01. ADR 3753. The audit test
`standard_encoding_is_no_predefined_encoding_spec_audit` passes and leaves
`expected-failures-current.json`; 2260-B01 is `fixed`; the D.1 row states all four `shall`s met.

- **Reader.** `glyph_names::PERMITTED_ENCODING_NAMES` holds Table 109's three names; a simple
  font's `/Encoding` or `/BaseEncoding` naming `StandardEncoding` is read as naming nothing and
  noted as the new class `font/encoding-name-standard` (clause ISO 32000-2 §D.1).
  `pdf-model`'s `type3::encoding` reads and notes a Type 3 font's name alike.
  `BaseEncoding::by_name` keeps the table for §9.6.5.1's and §9.6.5.4's fill.
- **Mapping.** `MAPPING` maps the class to `fonts/font-maps-read-as-written` and
  `fonts/non-symbolic-truetype-uses-a-standard-encoding`. The split is in the font-maps row, which
  knows the font: it passes over a repair placed where only nonsymbolic TrueType fonts naming the
  encoding are, and `pdf-transform`'s `font_maps` passes over those fonts by the same test, so
  `standard-truetype-encoding`'s rewrite and proof restate them. A Type 1 font's name is restated
  as read.
- **Writer.** `font_files/type1_names` no longer writes `/BaseEncoding /StandardEncoding`; it tries
  no base, then the three names Table 109 permits, and writes the first its code-by-code
  comparison holds for. With no base alone the census refused `issue215.pdf` (its program's own
  encoding fills codes the producer left unmapped); under `WinAnsiEncoding` it converts as before.
- **Tests.** The two archive tests round 2261's revert named pass unchanged. New: the class logged
  by `pdf-font` and by a Type 3 font in `pdf-model`; in `tests/archive/font_as_read.rs`, the
  TrueType font converts by the TrueType rewrite with the font-maps row passing, and a Type 1 font
  fails the font-maps row.

**Census.** `standard_encoding_census`: 4 documents, 4 fonts, one font's two codes differ
(`issue20232.pdf`, a TrueType font through `/BaseEncoding`, codes 169 and 180), as round 2261
found. The archive census with the pack, against 2261's: no line moves once `type1_names` tries
the named bases (before, `issue215.pdf` was refused on four profile lines). veraPDF 1.30.3: 5471
of 5523 compliant, 40 not, 12 unfinished, as 2261's. The run without the pack queued past the
round's budget behind other lanes' walks and was stopped, as 2261's was.

**Gates.** fmt, clippy, rustdoc, fuzz, `cargo test -p conformance` 0; nextest of the seven
packages: 2 failed, both in the manifest (2248-B03, B04). **Left.** A Type 3 font naming an unknown encoding is read as nothing without a note; the converter
does not restate a Type 3 font's maps, so a Type 3 font naming `StandardEncoding` is refused by the
output's own section 5.1 check.

**Trap or habit.** A doc-comment link to an item a macro declares (`crate::repair::X`) fails the
`names` gate though rustdoc resolves it: name the class by its string.
