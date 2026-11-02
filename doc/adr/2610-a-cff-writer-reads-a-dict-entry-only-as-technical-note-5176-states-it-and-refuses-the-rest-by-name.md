# 2610 — A CFF writer reads a DICT entry only as Technical Note 5176 states it, and refuses the rest by name

Session 1888, Slot 8 of batch one hundred and twenty-nine, 2026-10-29 and 2026-11-01.
Status: implemented and measured; two offset shapes the last fuzz run found are open (The fuzz finding).
Context: ADR 2578 (`with_glyphs_given`), ADR 2598 (`with_cids_given`),
the `cff_write` fuzz target. Code: `crates/pdf-font/src/cff.rs` (`exactly`, `dict_int`, `dict_two_ints`,
`operand_count`, `real_width`, `misstated_entry`, `offset_outside`, `Misstated`, `width_span`, `has_width`,
`charstring_widths`), `crates/pdf-font/src/cff_operand_tests.rs`, `fuzz/fuzz_targets/cff_write.rs`
(`advances_restated`).

## The question

The CFF writers here (`with_advances`, `with_glyphs_given`, `with_empty_glyphs`, `with_cids_given`, the
Font DICT repair) read a program's DICT offsets, rewrite it, and hand it to `read-fonts`, which draws it.
The fuzz target found programs on which the two read one DICT differently: a `CharStrings` entry of three
operands, a `Private` of five. The writer took the first operands, `read-fonts` takes the last, and the
written program named another INDEX than the one the writer had restated.

## What the note says, and what it does not

Adobe Technical Note #5176, section 4:

> An operator is preceded by the operand(s) that specify its value. A DICT is simply a sequence of
> operand(s)/operator bytes concatenated together.

and, of the bytes:

> Byte values 22–27, 31, and 255 are reserved. An operator may be preceded by up to a maximum of 48
> operands.

Tables 9, 10 and 23 give `charset`, `Encoding`, `CharStrings`, `FDArray`, `FDSelect` and `Subrs` one
number, `Private` two, and `defaultWidthX` and `nominalWidthX` one. The note says nothing of an entry stated
twice or with more operands than its table gives, and ISO 32000-2 is silent on the program's inside. Section
2 says of the layout that "the first five occupy fixed locations" — header, Name, Top DICT, String and
Global Subr INDEXes — with every other structure reached by offset. Section 4 writes a real as nibbles —
digits, a decimal point, an exponent, a minus sign — ended by a 0xf nibble, and reserves the nibble 0xd; it
says nothing of a nibble standing where that grammar admits none.

## What `read-fonts` 0.44 does

`ps/cff/dict.rs` pops the *last* operand for a one-operand entry and reads the first two for `Private`; a
later statement of an entry overwrites an earlier one; an integer entry stated as a real is skipped; and a
reserved byte clears the operand stack and is passed over ("Some buggy fonts have invalid dict operators").
`ps/num.rs` ends a real with the byte holding the first nibble its position does not admit — 0xd anywhere,
a second decimal point or exponent, a minus after the integer part — or in which an exponent past 1000 meets another digit,
and reads the next byte as the next operand.
Evidence about one reading, not the definition.

## The decision

1. **One shared reader.** `exactly::<N>` reads an offset only where the entry states exactly its table's
   count of integer operands; otherwise the entry has no value — never its first operands, never its last.
2. **A writer refuses, by name, a program whose followed entries are stated otherwise** (`misstated_entry`):
   twice, with another count, as a real where an integer is due — in the Top DICT, every `FDArray` Font
   DICT, and every Private DICT either names. A Font or Private DICT that does not tokenise — a reserved
   byte, an operand cut short — is `Misstated::Untokenised`: the writer cannot tell what it states, and
   `read-fonts` reads past the byte this reader stops at. Private DICTs holding together more bytes than the
   program are `Overlapping`, refused before any is read.
3. **An offset the rewrite cannot shift is refused** (`offset_outside`): one into the header, Name INDEX or
   Top DICT INDEX, or at or past the program's end. Every writer re-encodes the Top DICT, shifts each offset
   past it and appends what it rebuilt, so such an offset names other bytes afterwards.
4. **A width is inserted only where the count reads it** (`has_width`). Technical Note #5177 section 3.1
   makes the width positional: one operand more than the first stack-clearing operator takes. A width put
   before an `rmoveto` of one operand or an `endchar` of two would be read as an argument, and the glyph
   would keep `defaultWidthX`; such a glyph is `AdvanceNotRestatable`.
5. **An `FDArray` stated but not readable as an INDEX is refused** (`Misstated::FdArrayUnread`), where the
   rewrite had written it as an INDEX of no Font DICT and the reader then found none.
6. **A real ends where the reader ends it** (`real_width`, shared by `dict_entries` and `operand_count`).
   The writer restates what `read-fonts` will read, and on the note's silence about an inadmissible nibble
   that reader is the only one whose reading decides what the written program draws; a tokeniser ending a
   real only at 0xf took the entries after a reserved nibble into the number, so a second `nominalWidthX`
   went unseen and the writer reckoned against the first.

The refusal stays rather than reading as `read-fonts` reads, because the census shows it costs nothing:
the PDF/A census without the pack, on HEAD plus this change and again after each fix below, prints its 424 per-document lines identically
to HEAD's, the CFF and `Malformed` lines among them, and the test passes. Were a real producer's program
refused here, the remedy would be to read as `read-fonts` reads, with the refusal kept only where no
reading resolves the entry.

## The fuzz finding

`advances_restated` asks `with_advances` to restate up to sixteen advances and requires that
`charstring_widths` find every asked glyph at its advance and every other at the one it had. A ten-minute
fork-mode run (1 122 588 executions, 2 378 edges, 11 269 features) stopped 1 360 times on it; round 1882's
run stopped 335 times. The property was right and the writer wrong, six ways: a Private DICT holding a
reserved byte between two `nominalWidthX` (the writer read the first, the reader the last); an `rmoveto` of
one operand given a width the reader takes as `dx`; a `Private` or `charset` offset into the Top DICT INDEX,
shifted onto other bytes; a `charset` offset past the end, shifted into the appended `CharStrings` INDEX; an
unreadable `FDArray` rewritten as an empty one; a real holding the nibble 0xd, which hid a second
`nominalWidthX` from the writer. Each has a minimised test that fails with its fix planted away.

With the six fixes, the 1 709 artifacts of both runs and of the later runs replay as follows: 1 704 clean;
three of round 1882's stop at `cff_write.rs:185`, `with_cids_given`'s own read-back, not this writer's;
two stop at `cff_write.rs:91` on a charstring, not a DICT. Each such glyph ends a hint mask and then calls
`callsubr` on an empty stack, which `read-fonts` 0.44 answers with the stale value in the stack's first
slot; once a width is inserted, that slot holds the width and the call names no subroutine, so the glyph
that evaluated by accident no longer evaluates. That is a charstring-level disagreement, outside this
decision.

The ten-minute fork-mode run on the six fixes (1 386 836 executions, 2 462 edges, 11 919 features, 0.11 GiB
peak resident) stopped five times, none on a shape the six fixes address. Three are charstring-level: two
more of the empty-stack `callsubr` above, and a glyph that is only `seac` on an empty stack, which the
reader evaluates as `.notdef` plus `.notdef` and so as glyph 0's width, which the writer then restated. Two
are offsets the rewrite shifts and should refuse, and are left open: a Top DICT `Private` of negative size,
whose offset the restatement shifts (`top_dict`, `OP_PRIVATE`) so that a range the reader had dropped by
overflow becomes one it refuses; and in `Layout::with_glyph_tables`, a `Private` offset inside the String
INDEX, shifted by the whole growth where the bytes moved by less, so that the writer's output is refused
by its own second pass. Both belong beside `offset_outside`.

## The figures

`getrusage` of the test of a thousand overlapping Private DICTs: this change, 0.005–0.007 s wall and
11.8–12.6 MiB peak resident; the planted binary that reads each, 35.5–49.2 s and 36.9–37.3 MiB, then failing
its assertion. The six arms on HEAD plus this change are digest-identical by page to HEAD's (968 pages at
1x, 965 at 4x, in each of cpu, gpu and compute).
