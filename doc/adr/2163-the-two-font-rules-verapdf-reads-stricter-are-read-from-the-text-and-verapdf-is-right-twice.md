# 2163 — The two font rules veraPDF reads stricter are read from the text, and veraPDF is right twice

Session 1663, the archive slot of batch one hundred and three. Status: **accepted**; the validator
change each reading asks for is **owed**, and named below. Context: `doc/history/1655` "Found",
ADRs 2146, 2147; `crates/pdf-archive/src/table/fonts.rs`. Principle 5: veraPDF is evidence about
our reading, and the clause decides.

## 1. What veraPDF found that this validator passes

Round 1655 ran veraPDF 1.30.3 at 2b over 25 converted outputs. Two of its three failures were font
rules this validator passes, on fonts the producers embedded, so no conversion wrote them:

- `issue6410.pdf`, rule 6.2.11.2-6: "Widths array is missing or has invalid size 190 instead of
  224", four fonts (`PFDinTextPro-Thin` and three more).
- `wp315.pdf` (`doc/corpora/format-corpus/jhove-errors/PDF-HUL-133/`), rule 6.2.11.5-1: "Glyph
  width 318.010009765625 in the embedded font program is not consistent with the Widths entry of the
  font dictionary (value 317)", font `OOGFEA+dcbx100120`, codes 49 and 108.

The brief's hypothesis was that one of the two is veraPDF's misreading and one is ours. **It did
not hold**: both are ours.

## 2. 6.2.11.2-6: the `/Widths` length

ISO 19005-2 section 6.2.11.2 requires every font of a conforming file to conform to ISO 32000-1's
9.6 and 9.7. ISO 32000-2 §9.6.2.1's Table 109 (ISO 32000-1's Table 111 says the same) gives
`/Widths` its length:

> An array of ( LastChar - FirstChar + 1) numbers, each element being the glyph width for the
> character code that equals FirstChar plus the array index.

`issue6410`'s fonts state `/FirstChar` and `/LastChar` 224 codes apart and hold 190 widths. That is
a font dictionary that does not conform to 9.6, so section 6.2.11.2 fails exactly as veraPDF says.
This validator's row `fonts/font-programs-conform-to-their-own-specifications` is `Unchecked` as a
whole, on the argument that judging a *program* against its format needs a format validator. That
argument does not reach the dictionary half, which is a count. The row's reason now says so.

**Owed** (the next archive slot's): a requirement for the length, both parts (part 4's section
6.2.10.2 states the same rule), and its remedy. Restate `/LastChar` to `FirstChar + n - 1` for a
short array: the codes past it then take `/MissingWidth` by Table 109's next sentence, which is what
`pdf_font::metrics::stated_widths` already lays them out by, so no mark moves. Truncate a long one
to the range: Table 109 sends a code past `/LastChar` to `/MissingWidth` already. `stated_widths`
reads such a code from the array, though, which is a `pdf-font` defect for a pixels slot, to be
fixed first so the truncation moves nothing.

## 3. 6.2.11.5-1: the width of a program whose `FontMatrix` is not 1/1000

ISO 19005-2 section 6.2.11.5 asks the dictionary's widths and the program's to be consistent, and
defines consistent as differing by no more than 1/1000 unit. veraPDF applies 1 in glyph units of
1/1000 text space. This validator applies 0.001 in text space. That is the same tolerance.

The difference is in the program's number. `dcbx100120` is a Compact Font Format program
(`/FontFile3`) whose Top DICT states `FontMatrix [0.00049 0 0 0.00049 0 0]`, with `nominalWidthX`
1213. Glyph `G49`'s charstring opens with -564, so its width is 649 units, and 649 × 0.00049 =
0.31801 text-space units. `/Widths` says 0.317. They differ by 0.00101, which is more than the
thousandth, and the font fails. `pdf_font::cff::units_per_em` takes the reader's whole-number units
per em for this program. That is 1/0.00049 rounded, not 1/0.00049, so this validator reads the
advance a fraction of a unit short and passes it. The same scale draws the outlines, about one part
in twenty thousand small.

**Owed** (a pixels slot's, since `pdf-font` draws with it): units per em read from `FontMatrix` as
the real number it is, with the outlines and the advances both taken through it. The validator rule
then sees what veraPDF sees, and its doc comment names the misreading until it does. The converter's
`RestateFontMetrics` asks the same `program_advance`, so it will restate this program once the
reading is exact.

## 4. What did not change

No row moved status. `over` and `Unconsidered` stay 0. No validator verdict changed in this round,
because each correction belongs to a crate or a build this slot does not own. Each owed build has
its witness named above.
