# 3089 — A font retyped to Type1 ends the TrueType rows at it

Session 2097, an archive round in the `lane3` worktree, on the remedy the census of
`doc/pdfa-blocking-issues.md` (session 2094) ranks next: the step ADR 3078 left. Status: accepted
and built. In `pdf-transform`: `archive/font_files.rs` (`retyped_to_type1`, `without_retyped`,
`programs_named`, `truetype_rows`), `archive/prepare.rs` (the TrueType routes and the refiling read
the report `without_retyped` leaves; `Prepared::ended_by_retyping`), `archive/mod.rs`
(`met_by_the_retyping`). Tests: `crates/pdf-transform/tests/archive/truetype_over_cff.rs`
`a_truetype_row_failed_only_at_the_retyped_font_is_ended_by_the_retyping` (replacing
`a_font_a_truetype_row_also_binds_keeps_a_refusal_naming_it`) and
`a_truetype_row_failed_at_another_font_too_keeps_its_own_answer_for_that_font`; `font_files::tests::
a_truetype_row_keeps_only_the_findings_at_fonts_left_truetype`,
`a_row_no_truetype_clause_states_keeps_its_findings_at_a_retyped_font`. Context: ADR 3078 (the
retyping, and decision 3's refusal this replaces), ADR 2520 (the TrueType rows' restated `cmap`),
ADR 1942 (the standard route's names). Clauses: ISO 19005-2 sections 6.2.11.4.1 and 6.2.11.6,
ISO 19005-4 sections 6.2.10.4.1 and 6.2.10.6; ISO 32000-2 §9.9.1, §9.6.5.2, §9.6.5.4.

## Context

ADR 3078 writes a simple `TrueType` dictionary over a name-keyed CFF-flavoured OpenType program as
the `Type1` font Table 124 admits it under. ISO 19005-2 section 6.2.11.6 (ISO 19005-4 section
6.2.10.6) states seven requirements of TrueType fonts alone: their encodings, their `/Differences`
names and their programs' `cmap` subtables, all of them about §9.6.5.4's route from a code to a
glyph. Their rewrites (ADRs 1942, 2520) write a font as a TrueType font, so ADR 3078 refused a font
both would write, and `issue215` — whose `/Differences` names small capitals outside the Adobe
Glyph List — was the census's witness.

## Decision

1. **A retyped font is outside the TrueType rows' population.** Each of the seven binds a font
   whose `/Subtype` is `TrueType`; the output's font is `Type1`, reached by §9.6.5.2's names
   (§9.9.1's first bullet), so nothing those rows ask is asked of it. The retyping answers them at
   that font by removing the font from their subject, as a removal answers a row about what it
   removes (`actions::met_by_the_removal`).
2. **The fonts are known before the TrueType routes are prepared.** `retyped_to_type1` asks
   `rows_for` and the retyping's own proof (`type1_names::encoding`) of every program the refiling
   row named, without allocating an object, so the routes prepared before the packet can read the
   result. A font whose proof fails is not on the list, keeps every finding, and is refused by the
   refiling's own sentence.
3. **The report is narrowed, not the decision table.** `without_retyped` takes each TrueType
   row's findings at those fonts out of a copy of the report, reading the objects a row named
   whole (`Named::objects`), so a row that failed past the report's prefix of places is still
   ended when every object it named is a retyped font. The TrueType routes and the refiling read
   that copy; a row left failing at other fonts keeps its own answer for them.
4. **A row left with no finding takes the refiling's decision** (`met_by_the_retyping`):
   `Mechanical(FontProgramRefiled)` where the refiling proceeds, its refusal where it does not.
   Nothing is removed and nothing is stated, so every profile carries it, `refuse-any-loss`
   included.
5. **What cannot be told apart keeps a refusal.** A row whose named objects or places were cut
   short keeps its findings whole, and a font it reaches keeps ADR 3078's sentence
   (`A_TRUETYPE_ROW_TOO`, now worded for that case alone).

## Consequences

`issue215` converts at 2b and 4 under `as-shown` and `as-if-printed`; veraPDF 1.30.3 passes the
four outputs, and page 1 at 300 dpi is byte-identical to the source's. The census movement is the
round's record. The Type 1 program under `CIDFontType0` (`issue11740_reduced`) keeps ADR 3078
decision 4's refusal.
