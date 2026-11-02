# 2628 — A CFF writer refuses an offset below zero or into what it re-encodes, and an operator that pops an empty stack after the width

Session 1897, Slot 7 of batch 130, 2026-11-02.
Status: implemented; every stop of both ten-minute runs decoded and fixed (Fuzzing, below).
Context: ADR 2610 (the DICT operand rule and `offset_outside`), ADR 2578 (`with_glyphs_given`), ADR 2598
(`with_cids_given`), the `cff_write` fuzz target. Code: `crates/pdf-font/src/cff.rs` (`offset_outside`,
`Misstated::{InStrings, NegativeSize}`, `Layout::with_glyph_tables`, `pops_an_empty_stack`,
`charstring_with_advance`, `NotRestated`, `subroutine_body`, `SeacWalk`, `first_reserved_seac`), `crates/pdf-font/src/cff_cids_given.rs`,
`crates/pdf-font/src/cff_stop_tests.rs`.

## Decision

1. **An offset that names no predefined table is refused below zero as well as inside the bytes every
   writer re-encodes** (`Misstated::Outside`). `offset_outside` asked `at > -1` of `CharStrings`, `FDArray`,
   `FDSelect` and `Private`, so a negative offset passed. `read-fonts` 0.44 casts a Private DICT's two
   operands to `usize` and drops the Top DICT entry whose end overflows; a rewrite that shifted the offset
   by a positive growth brought it past zero and handed the reader a Private DICT inside the Top DICT INDEX.
   This was the HEAD run's artifact at `cff_write.rs:260` (Private 53 bytes at -48).
2. **A `Private` of negative size is refused by name** (`Misstated::NegativeSize`), whatever its offset:
   Technical Note #5176 Table 9 gives the operator a size and an offset, and whether the reader drops such
   an entry or refuses the font turns on the offset, which every writer moves.
3. **A writer that restates the String INDEX refuses an offset pointing into it**
   (`Misstated::InStrings`, checked in `Layout::with_glyph_tables`). The INDEX is re-encoded rather than
   copied, so its bytes move by less than the growth the offset is shifted by. `with_advances` copies the
   INDEX and still writes such a program.
4. **A charstring is not restated where, after its first operator and before any operand, it states an
   operator that takes operands** (`pops_an_empty_stack`, refused as `NotRestated::EmptyStack` by every
   writer through `charstring_with_advance`). `read-fonts`' `Stack::pop` answers slot 0 on an empty stack,
   the first operator clears the stack without overwriting slot 0, and slot 0 is where the width is
   written, so a width put in front changes the subroutine a `callsubr` calls or the codes a `seac` reads.
   Stem hints, masks, `endchar` and `return` take nothing off an empty stack and are passed over.

5. **A subroutine spliced into a charstring is its bytes before its first `return`** (`subroutine_body`).
   Technical Note #5177 section 4.7 sends the interpreter back to the caller at `return`, so a byte
   after it is never run; the splice that brings a width to the head kept every byte but a trailing
   `return`, so a subroutine with a `return` in its middle ran its dead tail in the restated glyph,
   which the reader then refused (two artifacts of the fixed run, glyph 2 read at no width).
6. **`with_advances` refuses by name a program in which any glyph reaches the escaped operator 12 6**
   (`first_reserved_seac`). Technical Note #5177's Appendix A lists 12 6 as reserved: Type 2 makes
   an accented character with `endchar`'s four optional arguments (section 4.1), and Type 1's `seac`,
   which 12 6 was, is not one of its operators. `read-fonts` 0.44 reads 12 6 as `seac` all the same
   and, where the glyph stated no width before it, gives the glyph its base character's width — the
   base being `.notdef` where the stack held no codes, the HEAD run's artifact `336bb699`. A glyph
   stating 12 6 therefore has an advance another glyph's charstring decides, so no restated advance
   can be said to keep it. Leaving only glyph 0's advance alone was rejected: the base is whatever
   glyph the codes name through the Standard Encoding and the charset, not only `.notdef`, so the
   narrower rule would hold for the one artifact and not for the operator. The walk follows every
   charstring through the subroutines it calls, its operand stack kept so that the subroutine called
   is the one entered and its stems counted across calls so that a mask's bytes are skipped as the
   reader skips them; it is bounded per glyph by `CHARSTRING_OPERATIONS` and over the program by
   32 000 000 operations. Whatever it cannot follow — a call on an empty stack, an operand an
   operator it does not compute left behind (Type 1's `hsbw`, `sbw` and `dotsection`, which the
   reader reads without clearing the stack, among them), more than Appendix B's 48 operands, a
   spent bound — counts as reaching 12 6, the side on which the writer refuses.
7. **A `Private` whose bytes end past the program's end is refused by name** (`Misstated::Outside`).
   Whether a rewrite's output holds the bytes such an entry states turns on the output's length: a
   Font DICT's Private read as absent in the first `with_cids_given` output read as 262 174 bytes of
   what followed it in the second, which its read-back refused (the brief's (c), `cff_write.rs:185`).
   `read-fonts` refuses such a Top DICT Private when it opens the program, and opens one a Font
   DICT states.
8. **A width is not inserted in front of a first operator that already takes 48 operands**
   (`NotRestated::FullStack`, `leading_operands`). A charstring stating no width whose first stem
   operator takes Appendix B's 48 operands came back with 49, and the walk of decision 6 run on the
   writer's own output read the forty-ninth as a point it cannot tell past and refused what its input
   had passed (`cff_write.rs:103`, artifact `806ce51b`: glyph 0 states 48 operands, `vstemhm`, a
   three-byte mask). `read-fonts` 0.44 holds 513 operands, CFF2's limit, and would read the output;
   the limit a Type 2 charstring is written under is still 48, so the writer refuses rather than
   write a charstring past it. Running the walk once on the input and carrying its verdict was
   rejected: the cause is a fault in the output, not in the walk, and carrying the verdict would
   have written it.
9. **A `Subrs` whose Local Subr INDEX does not read whole inside the program is refused by name**
   (`Misstated::SubrsUnread`). Decision 7's fault one table further on: the input's INDEX of 91
   subroutines ended 48 bytes past the program, so the walk of decision 6 read no subroutines and
   answered every call as one the reader stops at; the output, longer, held more of the bytes, and the
   same walk over it entered subroutines whose operators it cannot follow (`cff_write.rs:103`,
   artifact `e99f9c12`, glyph 3). What a charstring's calls reach may not turn on the length of the
   file it is written into.

## Fuzzing

The HEAD run (ten minutes, fork mode, 170 seeds) stopped twice, at `cff_write.rs:260` (decision 1) and
`:91` (decision 6). The run on decisions 1–4 stopped seven times: six at `:91` — four decision 6, two
decision 5 — and one at `:185`, decision 7. Every one of the nine replays clean on the code of
decisions 1–7, and the run on that code stopped after 89 s at `:103` on one artifact, decision 8.
The ten artifacts replay clean on decisions 1–8. The run on that code (ten minutes, fork mode with
`-ignore_crashes=1`, 1 737 seeds, 4 306 093 executions, coverage 2 697) stopped twice: at `:103`,
artifact `e99f9c12`, decision 9; and at `:185`, artifact `64656d0d`, which is the open question below.
All twelve but `64656d0d` replay clean on decisions 1–9; no run was taken on decisions 1–9.

## Open

`font_dict_shifted` re-encodes a Font DICT's `Private` and copies every other operand as it stands, so
a Font DICT that states `charset` (or `Encoding`, `CharStrings`, `FDSelect`, `FDArray`) past the String
INDEX keeps its value while the bytes it named move by the String INDEX's growth: artifact `64656d0d`'s
Font DICT, a copy of its Top DICT, states `charset 114` past a String INDEX ending at 112, and the
output's String INDEX, re-encoded with four-byte offsets, runs to 130. Whether the writer shifts those
operands or refuses them turns on what Technical Note #5176 section 19 lets a Font DICT state, which is
not yet read.
