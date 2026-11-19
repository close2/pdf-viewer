# 3155 — A Type 1 program under a `CIDFontType0` is converted to the CID-keyed CFF Table 124 files there

Session 2113, an archive round in the `lane4` worktree, on `doc/pdfa-blocking-issues.md`'s R10
entry for `issue11740_reduced`. Status: **accepted as a construction, not built**. Context: ADR
2716 (a `glyf` program under a `CIDFontType0`), ADR 3078 (a program re-filed under the key Table
124 names), `crates/pdf-font/src/composite.rs` (how this reader draws such a font).

## The case

`issue11740_reduced` states a `CIDFontType0` (`/CIDToGIDMap /Identity`, `Adobe-Identity-0`) whose
descriptor's `/FontFile` is a Type 1 program (`acAura-Bold`). ISO 32000-2 §9.9's Table 124 files a
Type 1 program under `/FontFile` for a `Type1` font only; a `CIDFontType0`'s program is a CFF under
`/FontFile3`, `/CIDFontType0C`, or an `OpenType` one. So ISO 19005-2 section 6.2.11.4.1 (the
program "embedded" as Table 124 files it) fails, and no re-filing of the bytes answers it.

This reader draws the font by §9.7.4.2's rule for a CFF without CIDFont operators, read through
§9.6.2.1 NOTE 1: the CID is the index of a charstring in the program's `CharStrings` order.

## The construction

1. Read the Type 1 program's charstrings in order (Adobe Type 1 Font Format chapter 6, decrypted
   under `lenIV`), with its `Subrs` and `OtherSubrs` calls.
2. Translate each charstring into the Type 2 form (Adobe Technical Note #5177): `hsbw`/`sbw` into
   the width operand and an initial `rmoveto` from the side bearing; `callsubr` inlined, except the
   hint-replacement and flex idioms of `OtherSubrs` 0 to 3, which become `hintmask` and `flex`;
   `seac` into `endchar` with its four accent operands (Type 2 keeps them); `div` kept;
   `closepath` dropped (Type 2 closes implicitly); `hstem3`/`vstem3` as plain stems.
3. Write a CID-keyed CFF (Technical Note #5176 section 18): one Font DICT with the Type 1
   program's Private DICT values, charset format 2 mapping CID *n* to GID *n* for every glyph, and
   `ROS` the CIDFont's own `CIDSystemInfo`.
4. File it under `/FontFile3 /Subtype /CIDFontType0C`, remove `/FontFile`, and drop
   `/CIDToGIDMap` (Table 115 states it for a `CIDFontType2` only).
5. **Prove before keeping**: the converted font, loaded through this reader, draws every CID the
   page shows with the outline the source drew (both read to `pdf_font::LoadedFont` outlines) and
   at the width `/W` states; a glyph that differs refuses by name.

Steps 1 and 2 are a Type 1 to Type 2 charstring converter, which this tree does not have
(`pdf_font::restate::type1_with_widths` rewrites `hsbw` in place and translates nothing); written
to standard it is a round of its own, which is why session 2113 records the construction rather
than building it. `issue11740_reduced` is the one census document behind it.
