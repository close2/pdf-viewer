# 1984 — A subroutine's body is found by running it

Session 1984, a single round, PDF/A lane's worktree. R09 (a)'s largest remainder and one defect
of the checker's; ADRs 2779 and 2780.

**The premise.** Half true. Round 1981's censuses refuse 25 documents at each part under `as-shown`
and `no-data-loss` with `MACHINE_FACE_NOT_RESTATABLE`, alone in 16 at 2b. The cause was not a width
pushed inside a subroutine as such — `restate` already splices a leading call — but the splice's
reader of the subroutine's body: it counted a hint mask's stems from zero and declined a body that
called another before its mask. And the census line is the *first* font refused, so freeing it
showed what else each document holds. The `/Intent` defect was not the survey's at all: the 32
images veraPDF names were held *exempt* (section 6.2.2), because the exemption lexed inline image
data as tokens and a sample byte `(` swallowed the `Do`s after it.

**Built.** ADR 2779 (`pdf_font::cff::subroutine_body`): the body is found by running the subroutine
as the interpreter would — nested calls run, the caller's operands and every stem counted, a mask's
data stepped over — and an operator the walk cannot follow exactly is declined. Every glyph of the
35 URW faces is now restatable (before, 9 and 12 of two faces). ADR 2780 (`pdf_archive::reach`):
`read_names` steps over an inline image's data at the end `pdf_model::inline_image::scan` finds; the
conversion is unchanged and restates the images to `RelativeColorimetric` by §8.6.5.8 (ADR 0957).

**Freed**: under `as-shown`, 6 R09 documents at each part (`176446`, `436840`, `436857`,
`AcroFormsBasicFields`, `issue9291`, `prefilled_f1040`) and `issue_cff_unsigned_bbox` (its own CFF
program's widths, refused under every profile before); under `no-data-loss` 3 at 2b and 6 at 4.
R09's row goes from 58 to 44: 11 of the other 19 stay on a second font drawn from a fitted CFF face
(`substitute-stretch`, 7 to 18), 8 leave for R08 and the rows they already stood in.

**Pixels** (CPU, 72 dpi, pages 1–4, `/OutputIntents` renamed): every freed `as-shown` output at 4 is
within one level of its source; `issue_cff_unsigned_bbox` is identical; `443752`'s changed output
draws the same pixels as before. Two pages looked at.
**Tests.** `cff_stop_tests.rs`'s `a_mask_counts_the_stems_the_caller_declares` fails with the
caller's operands not run, `a_width_behind_a_nested_call_and_a_mask_is_restated` with a nested call
declined; `tests/reach.rs`'s `a_name_after_an_inline_images_data_is_still_read` with the data lexed.

**Gates.** fmt 0 and clippy `-D warnings --all-targets` 0 on pdf-font and pdf-archive; nextest of
pdf-font and pdf-archive 0 (704); `cargo test -p conformance` 0 (471); censuses against
1981: shipped 16 NEW, 4 CHANGED, 0 GONE, stating 11 NEW, 3 CHANGED, 0 GONE; veraPDF 1.30.3 passes all
34 new and changed outputs and all 108 outputs of the 26 documents under every profile;
tier 2 (`corpus`, `raster_golden`) fails at HEAD and after alike, the same 16 pages; `state.sh docs`
exit 0, after the hand-off. No public signature changed: no fuzz check owed.
