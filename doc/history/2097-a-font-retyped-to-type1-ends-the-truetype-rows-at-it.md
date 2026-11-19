# 2097 — A font retyped to Type1 ends the TrueType rows at it

Session 2097, an archive round in the `lane3` worktree, on `doc/pdfa-blocking-issues.md`'s next
remedies. ADR 3089; ledger row 9.9.1 (`implemented`, before and after); no question.

**Built (a).** ISO 19005-2 section 6.2.11.6's seven TrueType rows bind TrueType fonts alone, and
ADR 3078's retyping writes a simple `TrueType` font over a name-keyed CFF program as `Type1`. The
fonts it retypes are found before the TrueType routes are prepared; those rows' findings at them
are taken out of the report the routes and the refiling read (reading `Named::objects` whole, so a
row failed past the report's prefix is still ended); a row left with none takes the refiling's
decision. `issue215` converts at 2b and 4 under `as-shown` and `as-if-printed`; veraPDF 1.30.3
passes all four outputs; page 1 at 300 dpi is byte-identical to the source's.

**Read, not built (b).** `issue12823`'s `/FT19` (`/DescendantFonts [ null ]`) fails only the
descendant row. The window draws nothing for it and does not move the text matrix (`show_text`
returns with no font; page 1 at 150 dpi is byte-identical with its two strings emptied). The
construction and the proof it owes (no `Tc` advance before the next positioning operator) are
written into the census's R10 entry; the proof needs a content walk the survey does not record,
and the round did not start what it could not finish with its gates and census.

**Census.** Re-taken by section 1's commands at this tree, with the pack and without. Against
session 2094's: pdf.js `as-if-printed` 2b converted 929 to 930 and refused 30 to 29, part 4 941 to
942 and 19 to 18 (`issue215`); pdf.js `as-shown` 2b 894 to 896 and 65 to 63, part 4 904 to 906 and
56 to 54 — `issue215`, and `issue15053` and `mixedfonts` freed while `90ms_rksj_h_sample` now stops
at section 6.2.11.4.1's CID 0 row, which is the machine-collection change between 2094's base and
this round's (`fa29fe4c`), not this round's. The corpora lines are unchanged.

**Tests.** `tests/archive/truetype_over_cff.rs`: the refusal test becomes
`a_truetype_row_failed_only_at_the_retyped_font_is_ended_by_the_retyping` (three profiles, two
parts), and `a_truetype_row_failed_at_another_font_too_keeps_its_own_answer_for_that_font` is
new; two unit tests of `without_retyped` in `font_files.rs`.

**Gates.** Over `-p pdf-transform`: fmt exit 0; clippy `-D warnings --all-targets` exit 0; nextest
`--lib --bins --tests` exit 0, no failure, none expected; rustdoc `-D warnings
--document-private-items` exit 0; conformance after `git add -N` exit 0. No public type changed.
