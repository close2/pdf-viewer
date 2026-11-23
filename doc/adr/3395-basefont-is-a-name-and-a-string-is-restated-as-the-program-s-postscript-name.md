# 3395 — `/BaseFont` is a name, and a string is restated as the program's PostScript name

Session 2173. Status: **accepted** and built. Context: ADR 3376 item 3, ADR 2846; ISO 32000-2
§7.3.5, §9.6.2.1 Table 109, §9.6.3, §9.7.4.1 Table 115, §9.7.6.1 Table 119, §9.8.1 Table 120;
ISO 19005-2 section 6.2.11.2, ISO 19005-4 section 6.2.10.2.

## The row

`fonts/base-font-is-a-name` fails a font or CIDFont dictionary whose `/BaseFont` is present and is
not a name: Tables 109, 115 and 119 give the entry the type name. A Type 3 font is not judged
(Table 110 states no `/BaseFont`); an absent entry is the required-entries gap, not this row's.
`issue1350` writes both fonts' `/BaseFont` (and the descriptors' `/FontName`) as UTF-16 strings
with a NUL before each letter; veraPDF fails them at 6.2.11.2 and 6.2.10.2, and no row read them.

## The remedy

`Rewrite::BaseFontNamed`, mechanical: the font's `/BaseFont` is written as the PostScript name
(name ID 6) its embedded `sfnt` program states, which §9.6.3 makes the value for a TrueType font,
and a descriptor's `/FontName` that is no name either is written the same, as Table 120 requires.
`issue1350`'s programs state the same malformed bytes as its strings; the OpenType specification
restricts name ID 6 to printable ASCII without ten delimiters, and the characters outside that
alphabet are left out, as `pdf_font::embed::postscript_name` reads it: `AACADT+ArialMT`,
`AACADS+Arial-BoldMT`. That is a reading of a malformed record, said as one; every character
written is the program's, in its order.

Refused by name: a font written inside another object, a Type 0 font (Table 119 composes its name
from its descendant's), a font whose program is not an `sfnt` or states no name, a descriptor that
is no object of its own.

## Measured

`issue1350` converts under both profiles at 2b and 4, and veraPDF 1.30.3 passes all four outputs.
