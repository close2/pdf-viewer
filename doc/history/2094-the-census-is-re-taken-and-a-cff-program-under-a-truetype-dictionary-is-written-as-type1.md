# 2094 — The census is re-taken, and a CFF program under a TrueType dictionary is written as Type1

Session 2094, an archive round in the `lane4` worktree, on `doc/pdfa-blocking-issues.md`. ADR 3078;
ledger row 9.9.1 (`implemented`, before and after); no question.

**The census.** Re-taken at `af46f367` by section 1's commands, with the CJK pack and without; the
two runs' census lines are the same. Against session 2036's: pdf.js `as-if-printed` 2b conforming
2 to 1 and refused 29 to 30, corpora `as-if-printed` converted one fewer at each part, and
`as-shown` refuses two more at every line but pdf.js part 4. The cause is no regression: ADR 2918's validator now reports a Type 3 code
with no glyph description, and `Type3Test` and `Type3WordSpacing` stop at R03a by that row where
they converted unchecked. `issue19634` and `issue19971` left R03 (ADR 2907); `0000490` and
`issue5470` joined R10 under `as-shown` (the layout-from-this-viewer stop). R08b no longer stops a
document with the pack.

**The pick.** By documents/alone, the first unbuilt reason is R10's
`fonts/font-programs-filed-by-their-format` (2/2 at 2b and 4, both profiles): `issue215`'s
CFF-flavoured OpenType program under a simple `TrueType` dictionary and `issue11740_reduced`'s
Type 1 program under `CIDFontType0`. The first needs no owner word; the second needs a format
conversion.

**Built.** The `TrueType` dictionary becomes `Type1` (Table 124's row, §9.9.1's first bullet),
its `/Differences` naming every code by the charset name of the glyph this reader drew it with,
proved by loading the retyped font and comparing all 256 codes; the program moves to `/FontFile3`
`/OpenType`. **Left**: `issue215` itself, whose small-capital `/Differences` names fail section
6.2.11.6's TrueType rows at the same font; their rewrites would write it as TrueType again, so it is
refused by a sentence of its own (ADR 3078 decision 3). The next step drops those rows' findings at a
retyped font from the input the TrueType routes read (`prepare.rs`).

**Evidence.** The fixture (`pdf-model`'s synthetic `cff.otf`) converts at 2b and 4 under `as-shown`
and `as-if-printed`; veraPDF 1.30.3 passes all four; page 1 at 300 dpi is byte-identical to the source's.

**Tests.** `tests/archive/truetype_over_cff.rs` (four); `a_program_no_row_admits_keeps_its_refusal`
now asks the Type 1 program under `CIDFontType0`, the case no row admits.

**Gates.** Over `-p pdf-font -p pdf-transform`: `cargo fmt --check` exit 0; clippy `-D warnings
--all-targets` exit 0; nextest `--lib --bins --tests` 1668 of 1668 passed (no audit failure expected
for either crate, none found); rustdoc `-D warnings --document-private-items` exit 0. Conformance
after `git add -N`: exit 0. No public type changed, so no fuzz check. The census runs exit 0, the
tier-2 walks are the train's.
