# 2540 — A Type 1 program's advance is read from its charstring, and an Adobe Glyph List name is a row of the 2002 table

Status: implemented. Session 1853. Q719 is not asked.
Context: ISO 19005-2 sections 6.2.11.5, 6.2.11.6 and 6.2.11.7.2, ISO 19005-4 sections 6.2.10.5 and 6.2.10.6 (cited, not
quoted); ISO 32000-2 §9.6.2.1 (Table 109); ADR 2520 section 8; `doc/pdfa-blocking-issues.md` section 8.
Code: `crates/pdf-font/src/type1.rs` (`Program::advance`), `loading/advances.rs` (`program_advance`),
`encoding.rs` (`is_listed`), `cff.rs` (`advances`, `nominal_width`); `crates/pdf-archive/src/finding.rs` (`Named`),
`table/fonts.rs`; `crates/pdf-transform/src/archive/sites.rs` (`objects_named`), `archive/fonts.rs`,
`archive/fonts/described.rs`.

## 1. The two readers that were too lenient

- **The program's width of a bare Type 1 font.** `LoadedFont::program_advance` answered `None` for every `/FontFile`
  program, so `fonts/widths-agree-with-the-program` could not fail one. A Type 1 charstring states its width as `hsbw`'s
  or `sbw`'s operand (Adobe Type 1 Font Format sections 6.4 and 6.5), and `read_fonts::ps::type1::evaluate_charstring`
  returns it unrounded; `type1::Program::advance` evaluates into a sink that keeps nothing, from the parse the loaded font
  already holds. Only the comparison asks for it; the page is still laid out by the dictionary's widths.
- **The names the list holds.** Both parts ask whether a `Differences` name (and, for the second exemption of 6.2.11.7.2,
  a referenced glyph name) is *listed* in the Adobe Glyph List they reference by its date, 20 September 2002.
  `encoding::text_for` is the list's *specification's* algorithm, which also derives text for `uniXXXX`, `uXXXX`, `_`
  ligatures and `.suffix` variants; none of those is a row. `encoding::is_listed` is the row test.

## 2. The table is `read-fonts`' own, and was compared with the 2002 file

Every row of the table is letters and digits, and none has the `uni`/`u` hexadecimal shape (counted over the 4 281
rows: 0). A probe in this session enumerated `read_fonts::ps::agl`'s trie through `char_to_nth_name` over every code
point and compared it with the 2002 `glyphlist.txt` (table version 2.0): the trie holds the 4 200 single-value rows and
no other name, and the 81 rows of several values answer `name_to_chars` exactly (0 mismatches). So `is_listed` is: the
name is alphanumeric, not of a derived shape, and `name_to_chars` answers. The ZapfDingbats and Symbol sets stay where
the clause names them.

## 3. Bound

`Program::advance` is one charstring evaluation per distinct shown code of a simple font (at most 256), and
`read_fonts::ps::cs` refuses a call nested past depth 10 before evaluating it (test at 10 and 11). It has no operation
budget: a subroutine calling the next k times per level costs k^9 calls — measured 0.07 ms at k = 2 and 1.3 ms at k = 3,
about 66 ns a call — and the renderer's `draw` and the CFF width reader share that exposure (report, Found, not fixed).

## 4. What moved, against the census of HEAD

Without the CJK pack the pdf.js lines go from 875 to 865, 881 to 871, 816 to 806 and 818 to 808 converted documents,
and the corpora lines from 398 to 398, 401 to 401, 336 to 335 and 333 to 332. Eleven pdf.js documents leave in each of
the four pdf.js lines: on the widths row, under `PROGRAM_NOT_RESTATABLE`, `bug1157493`, `bug1539074`, `bug1539074.1`,
`issue11150_reduced`, `issue1905`, `issue2833`; on the `Differences` row, refused by ADR 2520's restatement,
`bug1132849`, `issue6068`, `issue6889`, `issue6894`, `issue6961`. `ESUjulyaugust2002` leaves both `as-shown` lines on
the widths row. `javauninstall-7r`, refused at HEAD on the widths row, converts in all four pdf.js lines: its widths
are one unit from its program's, which section 5 makes consistent. `issue215` was refused already and now names the
`Differences` row first. With a copy of the pack the same documents move in the same lines, and the two
pdf.js `as-if-printed` lines the pack changes go from 876 to 866 and 882 to 872.

veraPDF fails HEAD's output of each of the twelve that left at both targets — at 6.2.11.5 test 1 (6.2.10.5 at part
4) on the widths row, at 6.2.11.4.1, 6.2.11.5 and 6.2.11.6 on the `Differences` row: 24 agreements, no disagreement.
It passes `javauninstall-7r`'s new output under both profiles at both targets, and `0004-637X_751_2_90`'s at both.

## 5. The CFF width, its nominal width, and the places a requirement names

- **A CFF width is read unrounded.** `cff::advances` evaluates each glyph's charstring and scales its width by
  `horizontal_metric` without `read-fonts`' `to_i32`, so `0004-637X_751_2_90`'s glyph 61 (`/FontFile3 /Type1C`) reads
  776.89 against `/Widths` 778 and fails 6.2.11.5 as veraPDF fails it; the converter restates the width. Every glyph of
  the ten shipped Foxit substitutes reads as the rounding reader read it (a test), so no width the viewer draws with
  moved.
- **A real `nominalWidthX` is floored as the evaluator adds it.** `cff::nominal_width` read operand 30 as 0, so a
  restated width read back larger by the whole nominal (778 written, 1 273 read in the witness); it now takes the
  dictionary's entry through `read-fonts` and floors it.
- **A Type 1 program that disagrees is refused by its own sentence.** `restate_metrics` refuses
  `PROGRAM_NOT_RESTATABLE` where a `/FontFile` program's `hsbw` width disagrees with `/Widths`, since no Type 1
  charstring writer exists.
- **A one-unit difference is consistent.** Six documents (`issue11045`, `3253-Article Text-6302-1-10-20190531`,
  `arsimon_1`, `joie12092-sup-0001-si`, `mattgib_1`, `meeram_1`) hold Type 1 programs whose `hsbw` widths are one unit
  below `/Widths` (543 against 544), and veraPDF passes each at both targets; 6.2.11.5 defines consistent as a
  difference of at most a thousandth of a unit, and `f32` stores 0.544 and 0.543 0.00100005 apart. The validator's
  `consistent` and the converter's `disagrees` compare in `f64` with a margin of four `f32` epsilons of each operand's
  magnitude, about 2.4e-7 of a unit at a width of 0.5 (test at the bound). Planted: `issue11045`'s code 58 written 273
  against the program's 271 fails both this validator and veraPDF at 6.2.11.5.
- **Every place a requirement names is known, or the rewrite is refused** (trap 289). The validator keeps the distinct
  objects a requirement names up to `pdf_archive::MAX_NAMED` (4 096) beside its 32 printed places, and says when it
  stopped short (`Named`); `sites::objects_named` refuses `CHECKER_CUT_SHORT` past the bound rather than rewrite a
  prefix.

Not settled here: the rows of 6.2.11.4.1 and 6.2.11.5 still reach a `uniXXXX` glyph of a non-symbolic TrueType font
through the reader's lenient name mapping, where 6.2.11.6's own mapping finds none (veraPDF fails a planted
`/uni00A7` at all three, this tree at 6.2.11.6 alone); `read-fonts`' charstring evaluator has no operation budget
(section 3); `fonts/truetype_cmap.rs` still refuses past 32 printed places where `Named` holds them all.
