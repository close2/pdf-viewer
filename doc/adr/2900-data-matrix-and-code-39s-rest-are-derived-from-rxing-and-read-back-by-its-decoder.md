# 2900 — Data Matrix, and Code 39's check character and extended set, are derived from `rxing` and read back by its decoder

Session 2042. Status: **accepted**, carrying out `doc/questions/A857` items 2 and 3. Clauses: ISO 32000-2 §K.2
(XFA 3.3 as the resource's format); XFA 3.3 pages 424, 437–442 and 611–620.

## What A857 changed

ADR 2877 built Code 39 from MIL-STD-1189A and refused what that text does not print — the modulo-43 check
character and `code3Of9extended` — and refused Data Matrix outright, whose encodations, field and placement are in
ISO/IEC 16022 alone. The owner answered Q857 that deriving normative tables from open source is allowed here. The
reading is narrow and the code keeps it narrow: **principle 5 is lifted for these symbologies only**, the source of
every table is named beside it with its licence, and nothing is copied from a copyleft project.

## The source, and why one

`rxing` 0.9.4 (Apache-2.0, a Rust port of `ZXing`) was already the dev-dependency that reads every symbol back
(ADR 2877), and its source is on this machine, so what the tree derives from is what a reviewer can open. Its Data
Matrix encoder is Jeremias Maerki's from `ZXing`, the one most other open implementations descend from. What was
taken, file by file, is in `crates/xfa-barcode/src/data_matrix.rs`'s comments and `doc/third-party-data.md`: the 24
square sizes, the six encodations' value tables, the pad and Base 256 randomisations, the generator's roots over
GF(2⁸) with `0x12D`, the interleaving, the utah walk and its four corner shapes, the finder pattern; for Code 39 the
43 check values and the 128-entry full-ASCII table.

**The decoder that reads the symbols back is the same project's**, which is the weakness worth naming (trap 9):
encoder and decoder could share a misreading. They share no code — the decoder's bit-matrix parser, data-block
splitter and bit-stream parser are separate files — and two fixed vectors come from outside both: ISO/IEC
16022:2000 Annex R's error codewords, as `rxing`'s own test carries them, and the size table's arithmetic, which
the tests hold against itself (each size's mapping matrix holds its codewords' bits exactly, or four bits over for
the corner pattern).

## The construction

- **Square symbols only.** XFA 3.3 page 442 fixes the ratio of rows to columns at 1:1, so the eight rectangular
  sizes are not carried.
- **One encodation a message, the shortest.** The encodation is the encoder's choice — every decoder reads every
  choice the same — so the choice is documented rather than derived: each of the six whole-message encodings is
  computed and the shortest taken, ASCII first on a tie. Switching modes inside a message (ISO/IEC 16022's
  informative look-ahead) would shorten some mixed messages and is not built.
- **Every end is written one way.** C40, Text and X12 always write the unlatch; a C40 or Text message one value
  short of a triplet gives bytes to ASCII until it is not, and two short is padded with shift 1; EDIFACT always
  writes its unlatch value, and the symbol is sized so that the three-codeword group it ends in fits; Base 256 always
  writes its length, and an empty message writes no field, because a length of zero means "to the end of the
  symbol". Each optional omission is an option, not a requirement, and taking none keeps one rule per encodation.
- **Bytes in.** Page 424 serialises the data as bytes before it is placed, UTF-8 unless `charEncoding` says
  otherwise, so `encode_matrix` takes bytes and the caller serialises; `dataPrep="flateCompress"` (page 614) is the
  caller's too.
- **Code 39's `checksum`.** Page 440 ticks `auto` and `none` for `code3of9` and `code3of9extended`, so `auto`
  appends the modulo-43 character and the other keywords stay refused; it ticks nothing for `logmars`, and page 613
  says the attribute is then ignored, so a `logmars` symbol is the same whatever it says.
- **Arithmetic is bounded before it is done.** Each module states its bound in one `expect` reason — a message past
  what the largest symbol can carry is refused up front — and the new `xfa_barcode` fuzz target holds the claim, as
  ADR 0096 asked of every such reason.

## What is left

The layout that calls the crate (`doc/todo/78` section 4) is not built; nothing draws a symbol yet. Mixed-mode
Data Matrix encoding is not built. Data Matrix's ECI, FNC1, macro and structured append — none of which XFA's
template can ask for — are not built.
