# 3173 — A Type 1 program under a `CIDFontType0` is converted by the path its interpreter draws

Session 2118, an archive round in the `lane3` worktree. Status: **accepted** and built. Builds the
construction of ADR 3155 for `doc/pdfa-blocking-issues.md`'s R10 entry `issue11740_reduced`, and
departs from its step 2 and step 3 where this records. Context: `crates/pdf-font/src/type1_to_cff.rs`
(`pdf_font::type1::as_cid_keyed_cff`), `crates/pdf-transform/src/archive/font_files.rs`
(`as_cid_keyed`), ADR 1486 (a `seac` composed into one outline for a subset).

## 1. Each charstring is translated through the interpreter the window draws with

ADR 3155 step 2 translated operator by operator: `callsubr` inlined, `OtherSubrs` 0 to 3 turned
into `flex` and `hintmask`, `seac` kept as `endchar`'s four operands. The tree has no Type 1
charstring reader of its own: `pdf_font::type1` reads every program through `read_fonts::ps`, and
that interpreter is the definition of what this reader draws. So each charstring is *run* through
it, into a `CommandSink` that writes what it is handed as `rmoveto`, `rlineto` and `rrcurveto`, each
point relative to the last in Technical Note #5177's 16.16 operand form, after the width `hsbw` or
`sbw` stated and before `endchar` — the writer ADR 1486 already uses (`embed::accented::Path`).
`callsubr`, `div`, flex and the side bearing arrive as the points they compute; `closepath` is the
note's implicit close. **`seac` is written as its two components' paths** and not as `endchar`'s
accent form, because that form reaches its components through `StandardEncoding` and a charset of
glyph names, and a CID-keyed charset holds CIDs (ADR 1486's argument, for the same reason).

What this buys: the converted glyph is the outline the window drew by construction, and the proof
below can demand bit equality. What it costs: the shape of the program's own operators is not kept
(two `rrcurveto`s where the source said flex, which only matters to a renderer that would have
drawn a flex below its height as a line; the interpreter here draws curves either way).

## 2. Stem hints are not carried, and the Private DICT states none

The interpreter reports Type 1's `hstem3` and `vstem3` and the hint replacement of `OtherSubrs` 3 to
no sink (`read_fonts::ps::cs`), and reports `hstem`/`vstem` from the side bearing rather than the
origin Type 2 measures from. Stems read through it would be a subset of the program's, and a Private
DICT carrying the program's blue zones over a subset of its stems would hint worse than none. So the
charstrings carry no hints and the Private DICT states only `0 nominalWidthX`. Hints decide how an
outline is fitted to a coarse pixel grid and never the outline itself; this reader does not hint.
The cost is a hinting renderer's small sizes, said in the rewrite's own sentence. Carrying hints
needs a Type 1 hint reader of this tree's own (`hstem3`, `vstem3`, `OtherSubrs` 3, the side bearing)
writing `hstemhm`/`vstemhm` and `hintmask`; it is left.

## 3. The proof, and the refusals

The written program is read back by `pdf_font::cff`: its charset must send every CID *n* to glyph
*n*, its units per em must be the source's, and **every** glyph (not only those a page shows) must
draw command for command and coordinate bit for bit as `pdf_font::type1::Program::draw` drew it,
at the same charstring width. Refused by its glyph index and name: a charstring the interpreter
refuses, a point farther from the last than 16.16 states, a glyph whose read-back differs, and **a
charstring stating no width** — a Type 2 charstring that omits its width has `defaultWidthX`
(TN #5177 section 3.1), so writing one would state a width the source never stated. Refused whole:
a `FontMatrix` other than the 1000-unit one (the Top DICT states none and takes the default), more
than 65 535 glyphs, no `/FontName`. In the archive, also refused: a `CIDFontType0` written inside its
`Type0` font, one stating no `/CIDSystemInfo` (the program's `ROS`), or two naming one program and
disagreeing about it. Each archive refusal is the one sentence `NOT_CONVERTED`.

## 4. Filing

The program becomes a whole new stream under `/FontFile3 /Subtype /CIDFontType0C` (Table 124),
the descriptor's key moves, and the `CIDFontType0` loses `/CIDToGIDMap` (Table 115 states it of a
Type 2 `CIDFont` alone) through the `retyped` path ADR 2326 built. `/W` is untouched: the converted
program states the widths the source did.

## 5. Found, not fixed

`pdf_font::cff::charstring_widths` reads a Type 2 charstring that omits its width as stating none,
where TN #5177 section 3.1 gives it the Private DICT's `defaultWidthX` (0 by Table 23's default).
A fixture written with such a glyph conformed to this tree's validator and failed veraPDF 1.30.3 at
ISO 19005-2 6.2.11.5; the standard reads with veraPDF. Left for a round on §9.6.2's width check.
