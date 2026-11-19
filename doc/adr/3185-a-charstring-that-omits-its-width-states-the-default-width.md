# 3185 — A Type 2 charstring that omits its width states the Private DICT's `defaultWidthX`, 0 where the DICT is silent

Session 2121. Status: accepted. Rows: §9.6.2.1 (`implemented`, held; three tests added). Context:
ADR 3173 section 5 (where this was found), ADRs 2540 and 2610 (the width reader and the writer's
operand rule), ADR 0953 (which of a font's two widths may move).

## The rule

Adobe Technical Note #5177, section 3.1, is the CFF program's normative reference for a glyph's
width:

> 1) Width: If the charstring has a width other than that of defaultWidthX (see Technical Note
> #5176, “The Compact Font Format Specification”), it must be specified as the first number in the
> charstring, and encoded as the difference from nominalWidthX.

Technical Note #5176 section 15 says how it is read back — "If a glyph width equals the
defaultWidthX value it can be omitted from the charstring, otherwise the glyph width is computed
by adding the charstring width to nominalWidthX value" — and its Table 23 gives both operators the
default 0. Both are Private DICT operators, so a CID-keyed program reads them from the Font DICT
`FDSelect` names for the glyph (sections 18 and 19). Neither note is held in `doc/`; the copies
read are Adobe's published PDFs, fetched for this round.

## The defect

`read-fonts` evaluates a charstring to `width + nominalWidthX` where the charstring states one,
and otherwise to the subfont's `defaultWidthX` as an `Option` — `None` where the Private DICT does
not state the operator. `pdf_font::cff::evaluated_widths`, behind both `advances` and
`charstring_widths`, passed that `None` on as "the program states no width". Table 23's default
makes it a statement of 0. A program stating a glyph that way conformed to this tree's ISO
19005-2 6.2.11.5 check (which passes over a glyph whose program width is `None`) and failed
veraPDF 1.30.3 at the same clause; the notes side with veraPDF.

## The fix, and who reads it

`evaluated_widths` reads the evaluator's `None` as 0 and nothing else changes: `read-fonts` already
reads `defaultWidthX` and `nominalWidthX` per subfont, so a non-zero default and the two-Font-DICT
case were right before and are now held by tests (`cff_width_tests.rs`). `None` remains the answer
for a glyph whose charstring cannot be evaluated or is not there.

| consumer | before | after |
|---|---|---|
| `pdf-archive`'s 6.2.11.5 rule (`widths_agree_with_the_program`) | glyph passed over | judged against 0 |
| `pdf-transform`'s width remedy (`program_statement`) | no restatement | the program's width restated to the dictionary's |
| `metrics::simple_widths`, a font with no `/Widths` outside the fourteen | resembling standard width, else `/MissingWidth` | the program's 0 |
| `restate`'s OpenType `CFF ` check, `with_glyphs_emptied`, `stretch_cff`'s proof | `None` | 0, compared like any other |
| the substitute faces (`metrics::substitute_stretch`, `.notdef` advance) | — | unchanged: no glyph of the ten compiled-in bare CFF faces omits its width under a silent DICT (probed) |

The third row is a rendering change, for a malformed shape only (Table 109 requires `/Widths`
outside the fourteen): the window now advances such a glyph by what the program states.

## Not changed

`read-fonts` floors `defaultWidthX` and `nominalWidthX` to whole units on read, where Table 23 makes
each a number; `with_advances` states a width against that same floor (`nominal_width`), so reader
and writer agree. A fractional operator in a real program would make this reader's width differ
from the notes' by under one unit; that is left as its own question, not taken here.
