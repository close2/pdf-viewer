# 2677 — A character WinAnsiEncoding does not name is set through a Differences array

Session 1927, a single archive round.
Status: accepted and built in `pdf-transform` (`archive/preserve.rs`: `encoded_by_name`,
`listed_glyph_name`, `font_dictionary` writing the encoding it is given; `NO_FACE` restated).
Context: ADR 1014 section 5 and `doc/questions/A47` (a face this program ships), ADR 2676.
Clauses: ISO 32000-2 §9.6.5.4, Table 112, §9.3.3; ISO 19005-2 section 6.2.11.6, ISO 19005-4 section
6.2.10.6 (`fonts/non-symbolic-truetype-differences-are-listed-names`,
`fonts/non-symbolic-truetype-differences-need-the-unicode-cmap`).

## The premise, checked

The R14 paragraph said the shipped faces "cover WinAnsi only" and the CJK pack would set the rest.
Neither half held for the two documents. `issue20453`'s packet holds one character outside WinAnsi,
`ё` (U+0451), and Liberation Sans draws it: the limit was the encoding the appended page's simple
font was written under, not the face. `balloon_a1b_jp2k`'s holds U+0000 in its packet header's
`begin` attribute, which no face draws and no pack covers. The CJK pack is a second download
(`doc/cjk-pack.md`), not part of the tree.

## Decision

The appended page's shipped face keeps `/WinAnsiEncoding` by name wherever every character has a
WinAnsi code, so no output that converted before changes. Otherwise its encoding is a dictionary
over `/WinAnsiEncoding` whose `/Differences` array gives each remaining character a code the text
does not use (never 32, which §9.3.3's word spacing reads) under the name the Adobe Glyph List lists
for it. §9.6.5.4 maps such a code to the name, the name to Unicode through the list, and that to a
glyph through the program's (3,1) subtable — the route a nonsymbolic font's encoding dictionary
takes — and ISO 19005-2 section 6.2.11.6 admits exactly this array over that subtable. The code table
is built by `pdf_font::standard::shipped_face` from the same dictionary, so the glyphs written are the
ones a reader selects. A character with no listed name (a computed `uniXXXX` is refused), or more
characters than free codes, keeps `NO_FACE`.

## Rejected

**A Type 0 font over the same program**, which §9.6.5.4's NOTE 2 names for glyphs a simple font
cannot reach and whose guideline prefers a nonsymbolic font without `/Differences`. It would reach
every glyph, but §9.9 says a TrueType program under a CIDFont "shall not" carry a `cmap` table, so the
program would be rewritten as well as a CIDFont, a `/W` array and a two-byte `ToUnicode` added. A
guideline is a *should*; the array is a route both parts of ISO 19005 name. Revisit when a document
needs more characters than a simple font has codes.
