# 3281 — ASCII data that ends before its end-of-data marker is truncated

Status: accepted. Session 2145. Context: ISO 32000-2 §7.3.8.2, §7.4.2, §7.4.3; audit 2142
(`doc/reviews/2142-specification-audit-7-4.md`) finding B02; `Damage::Truncated`; ADR 0587, ADR 2438.

## The defect

`ascii_hex` and `ascii85`, and the windowed `AsciiHex` and `Ascii85` stages, reported data that ran out before `>`
or `~>` as a whole decode, and applied the end-of-data rules to it — the odd hex digit padded with a zero, the
partial base-85 group padded and flushed. RunLength, LZW and Flate report the same case as `Damage::Truncated`, and
`Decoded::damage` is documented `None` only where the decode reached the filter's end-of-data. A cut ASCII-armoured
stream was drawn as if complete.

## Decision

1. Both filters, on both routes, report `Damage::Truncated` when the input ends before the marker. The bytes decoded
   up to the cut are handed over, as for the other filters.
2. The end-of-data rules are not applied at a cut. §7.4.2 pads an odd digit "[i]f the filter encounters the EOD
   marker", and §7.4.3 makes a partial group the encoding of the last *n* bytes only where the marker immediately
   follows it. Without the marker the digit is half a byte and the group a group the cut ended, whose padded value
   names bytes the file never stated; both are left out.
3. A base-85 stream that ends on a lone `~` is the same truncation.

## Cost

A file whose ASCII stream omits its marker now carries a damage report beside the same bytes, less a final odd digit
or partial group. Where a consumer refuses a damaged decode (a cross-reference stream, a font program), such a
file is refused where it was read whole; that is the reading §7.3.8.2's "[a]ll of these constraints shall be
consistent" gives a stream whose filter says it was cut.

In the tracked corpus one document reports it: `PDFBOX-3148-2-fuzzed.pdf` writes every `/A85` stream without `~>`,
and its widget's appearance stream `03!0=6;` is now drawn from its whole group with the cut reported; it is admitted
to the corpus walk's `INCOMPLETE` list as the file's.
