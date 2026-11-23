# 3426 — A CFF real with no digit is restated as the zero this reader read

Session 2181. Status: **accepted** and built; ADR 3412 item 3. Context: ISO 32000-2 §9.9's Table
124; Adobe Technical Note #5176 section 4; ISO 19005-2 section 6.2.11.2, ISO 19005-4 section
6.2.10.2; ADR 2610 (`pdf_font::cff::real_width`).

## Reading

Technical Note #5176 encodes a real DICT operand as 30 and a decimal number's nibbles ended by 0xf.
`bug1068432`'s Top DICT states UnderlinePosition and UnderlineThickness as `1e ff`, and
`bug1308536`'s Private DICT states BlueValues operands as `1e fc` and as `1e c5 77 02` (an exponent
and no mantissa): no digit, so no number. veraPDF reads the nibbles as a decimal string and refuses
the whole program ("Invalid number format: empty String"), then fails 6.2.11.4.1 / 6.2.10.4.1 as if
nothing were embedded. `read-fonts`, which draws every CFF glyph here, composes the value from its
digits after FreeType's `cff_parse_real` and returns **zero** where it composed none, ending the
operand at the byte holding the 0xf — or the exponent digit past 1000 — where it stops.

## Decision

1. **A finding.** `fonts/compact-font-reals-state-a-number` (6.2.11.2 / 6.2.10.2) reports a bare
   `Type1C` or `CIDFontType0C` program whose Top, Font or Private DICT holds such a real
   (`pdf_font::cff::digitless_reals`, tokenised as `read-fonts` tokenises, an unknown operator byte
   one byte). A program `/OpenType` wraps is not read: its table checksums would need restating too.
2. **The remedy writes the value this reader read, in the operand's own bytes.**
   `Rewrite::CompactFontRealsRestated` overwrites each such operand with zero digits ended by 0xf
   over the width `read-fonts` read it at, so no offset moves and every other byte is unchanged;
   a decimal-string reader then reads the same zero over the same bytes. `pdf_transform`'s
   `compact_reals` draws every glyph of both programs and refuses by name unless every outline is
   identical, segment for segment.
