# 2685 — A Type 1 program's widths are restated in its charstrings

Session 1932, a single archive round.
Status: accepted and built. In `pdf-font`: `type1_widths.rs` (`type1::with_advances`) and
`restate.rs` (`type1_with_widths`, `type1_advance`); `type1.rs` (`horizontal_scale`,
`horizontal_advance`), which `LoadedFont::program_advance` now reads; the scan in
`type1_glyphs_given.rs` records each entry's byte count. In `pdf-transform`: `archive/fonts.rs`
(`Filed`, `type1_restated`; `embedded_program` now answers `/FontFile`). Tests: `type1::glyphs_given::widths::tests` (five)
and the two of `crates/pdf-transform/tests/archive/type1_widths.rs`.
Context: ADR 0947 (restate the program, never the dictionary), ADR 0988 (prove before writing),
ADR 2540 (a Type 1 advance is read from its charstring, and the refusal this replaces), ADR 2620
and 2650 (the Type 1 writer this reuses), ADR 2640 (a shared program is restated in a copy),
ADR 2490 and `doc/pdfa-blocking-issues.md` R10.
Clauses: ISO 19005-2 sections 6.2.11.2 and 6.2.11.5, ISO 19005-4 sections 6.2.10.2 and 6.2.10.5;
ISO 32000-2 §9.2.4, §9.6.2.1, §9.9, §9.9.1 (Table 125); ISO 32000-1 Table 111; Adobe Type 1 Font
Format sections 2.4, 6.2, 6.4, 7.1 to 7.3.

## Context

Both parts require the widths a font dictionary states and the widths its embedded program states
to agree. §9.2.4 says the dictionary's are what position a glyph, so the converter restates the
program (ADR 0947) and never `/Widths`. That was built for an sfnt and a CFF. A Type 1 program
under `/FontFile` was refused by name (ADR 2540): its widths are inside charstrings that are
encrypted twice. The re-ranked census put R10 first among the rows that need no owner's word, and
this refusal was its largest part: eight documents, seven of them by a Type 1 program (six pdf.js,
one corpora) and one by a CFF width inside a subroutine. veraPDF fails `bug1157493` and `issue2833` at
6.2.11.5 before conversion, which is the premise checked.

## Decision

1. **The width is the `wx` operand of the `hsbw` (or `sbw`) that opens the glyph's charstring**,
   which Adobe Type 1 Font Format section 6.4 makes every charstring's first command. Only that
   operand is written. The `lenIV` leading bytes keep the producer's values, and every other byte
   of the charstring is kept. A charstring that does not open with either command over plain
   numbers is refused with the existing sentence, now naming that case.
2. **The program keeps its structure.** The `eexec` section is decrypted, each named entry gets
   its restated charstring and its new byte count before `RD`, and the section is encrypted again.
   Every other entry is copied byte for byte. The header is written as ADR 2650 writes it, the
   trailer is copied, and the stream states the three lengths again (§9.9.1, Table 125).
3. **Proved twice before it is written** (ADR 0988, trap 301). The writer's scan reads every
   untouched entry back unchanged. `read-fonts` then reads the same glyph names, the asked advance
   for each named glyph, and the same outline as before. The converter also reads each width back
   in ems against the dictionary's, within the 0.001 both parts allow.
4. **A width is the horizontal component of the advance, carried through the `/FontMatrix`.**
   §9.2.4 calls a glyph's width "formally, its horizontal displacement". Section 6.4's vector
   (`wx`, 0) reaches text space through the matrix, so its horizontal component is `wx` times the
   matrix's first element. `read-fonts` scales outlines by the fourth element. A matrix that
   extends a face (`[0.0005 0 0 0.001 0 0]`, `bug1539074`) or skews it
   (`[0.001 0.002 0.003 -0.004 0 0]`, `bug1539074.1`) therefore has a width unit different from
   its height unit. `type1::Program::horizontal_scale` is that ratio, and
   `LoadedFont::program_advance`, the checker's reading, now carries it. The first build compared
   raw `wx` against the vertical units per em, and veraPDF failed both documents' outputs at
   6.2.11.5; with the ratio, both pass. A matrix with no horizontal scale, or a mirrored one, is
   refused.
5. **A dictionary that names a descriptor and no widths is refused by name.** ISO 32000-1
   Table 111 lets only the standard 14 fonts omit `/FirstChar`, `/LastChar` and `/Widths`, and
   for those it makes the four entries, `/FontDescriptor` included, all present or all absent. `issue11150_reduced`'s `Symbol` states a descriptor and no widths. Restating its program
   alone gave an output that veraPDF fails at 6.2.11.2-4, -5 and -6, and veraPDF is right. The
   remedy, writing the three entries as the widths this reader positions by, is not built. This
   tree's own checker does not fail the source on that rule (see the round's record).
6. **The rest is the route a CFF takes**: the glyphs whose two statements disagree, from the
   strings the pages show, rounded to the program's `FontMatrix` em or refused as too coarse; a
   program another font shares is restated in a copy (ADR 2640, which now finds `/FontFile` too).

Nothing a reader sees moves. §9.2.4 positions by the dictionary, and section 6.4 makes `sbx` the
outline's origin and `wx` only the next glyph's. Page 1 of `bug1157493` and of `issue2833`
renders to the same PNG before and after conversion.

## What is left of R10

A CFF width inside a subroutine (`issue_cff_unsigned_bbox`), a `glyf` program under
`CIDFontType0`, a `CIDSystemInfo` that disagrees with its CMap, a descriptor or CIDFont written
inline, two codes on one glyph at two widths, a Type 0 font with no descendant, and a Type 1
program under a dictionary with a descriptor and no widths (`issue11150_reduced`, decision 5). Each
is still refused by its own sentence.
