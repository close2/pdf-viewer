# 2642 — An OpenType program with PostScript outlines states each advance twice, and both are read and restated

Status: implemented — session 1904, slot 4 of batch one hundred and thirty-one. Context: ADR 2630's
item 4 (an OpenType-CFF face supplied through `--font` converts, this tree's checker passes it, and
veraPDF fails ISO 19005-2 section 6.2.11.5 on every glyph restated); `doc/pdf-a-conversion-limits.md`
section 4.9 (the program is the side that moves); ADR 0988 (a restatement is proved by reading it back).

## The finding, and why both statements are the program's

ISO/IEC 14496-22 states an advance in `hmtx`, and an OpenType program with PostScript outlines sends a
reader to its `CFF ` table, which is a whole CFF program whose Type 2 charstrings each state a width
as well (Adobe Technical Note #5177 section 3.1). ISO 19005-2 section 6.2.11.5 and ISO 19005-4 section
6.2.10.5 ask the *font program* to agree with the dictionary and do not name a table; ISO 32000-2
§9.2.4 says "The width information for each glyph shall be stored both in the font dictionary and in
the font program itself", and §9.6.2.1's Table 109 holds `/Widths` to "the actual widths given in the
font program". So neither table
is the program's statement to the exclusion of the other, and a file is consistent only where both
agree with `/Widths` or `/W`.

`pdf_font::restate::with_widths` rewrote `hmtx` alone for an sfnt, and `LoadedFont::program_advance`
read `hmtx` alone. Run at HEAD, the Greek document of ADR 2630 converted with Noto Serif as an
OpenType-CFF face passed this checker and failed veraPDF's 6.2.11.5 (part 2) and 6.2.10.5 (part 4)
with 375 failed checks each.

## The decision

1. **The checker reads both.** `LoadedFont::program_charstring_advance` answers the `CFF ` table's
   width, in the table's own `FontMatrix` scale (which need not be the reciprocal of `head`'s
   `unitsPerEm`), and `fonts/widths-agree-with-the-program` names a glyph whichever statement
   disagrees, under its own sentence for the charstring case.
2. **The restatement writes both.** For an sfnt carrying a `CFF ` table, `with_widths` restates
   `hmtx` as before and the table's charstring widths through `cff::with_advances`, the same writer
   a bare CFF takes; the file is laid out again (`sfnt::with_tables`) rather than appended to, since
   the table is every outline and an appended copy would carry them twice, and every checksum is
   restated.
3. **Every sfnt restatement is read back** before a caller is handed it: the restated file is
   checksummed again and must come back byte for byte (`sfnt::checksums_hold`, the fixed point of
   `sfnt::checksummed`), and each named glyph's advance is read through `restate::advance`, a
   refusal being `RestateError::NotReadBack`. The fixed point, and not a second summation, because
   a producer's `head` need not sit on the four-byte boundary ISO/IEC 14496-22 asks for: there
   `checkSumAdjustment` straddles two of the file's words, and only the arithmetic that wrote it
   reproduces it.
4. **`restate::advance` answers `None` for an OpenType program whose two statements disagree**, so
   every converter route asking "does this face already state the file's width" restates such a
   face rather than trusting `hmtx`; the embedded-program route asks the same of a loaded font
   (`program_statement` in `archive/fonts.rs`).

A `glyf` program is untouched: `hmtx` is its only statement. A `CFF2` table states no charstring
widths and is not read.

## Evidence

Converted at HEAD and with the change, `--font Times-Roman=` an OpenType-CFF Noto Serif (the OFL face
ADR 2630 fetched): HEAD fails veraPDF at 2b and 4,
375 checks each; the change passes at both, its 23 pages drawn identically to HEAD's output, page for
page, and its report unchanged. HEAD's own output, archived again by the change, is named by the checker
(90 places), its embedded program restated through the embedded route, and passes veraPDF at 2b.
The plant: the `CFF ` record's checksum written stale after the restatement, and the read-back refuses
the program (two tests fail with `NotReadBack`); the checker without its charstring reading fails the
new fixture test.

## Consequences

- The read-back's first form summed each table and the adjustment over aligned words, and refused
  two corpus documents the census had converted at HEAD, each with a `glyf` program a producer laid
  out with `head` at an unaligned offset (pdf.js's `issue3323.pdf`, `head` at byte 34 038, and the
  corpora's `363_Risk-Based ... Paper.pdf`). The fixed-point form reads both back, and
  `a_head_off_the_word_boundary_reads_back` holds it.

- `cff::with_charstrings` appends the restated `CharStrings` INDEX and leaves the old one unreferenced,
  so a restated unsubroutinised face doubles (the probe's `CFF ` table went from 811 902 to 1 589 054
  bytes, the output from 3.08 MB to 3.35 MB). A bare CFF already paid this; compacting it is the CFF
  writer's work and not this decision's.
- The checker's charstring reading opens the table once per code shown, the cost a bare CFF's width
  reading already has.
