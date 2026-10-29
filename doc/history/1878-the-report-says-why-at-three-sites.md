# 1878 — The report says why, at three sites

Slot 8 of batch 128, 2026-10-29, an "archive" round (Opus 5.5), two sessions; ADRs 2590 and 2591, Q754 unused.
I worked in `scratchpad/r1878/` and one export of this tree built there. No ledger row moved: the slot is
`doc/rfc/0014` section 3's ("the report names each object's step") and principle 1's.

**(a) A statement handed on says why.** At a halftone row whose `append-statement` hands its objects to `discard`
(ADR 2562), the composition's sentence was dropped and the report said `discard` alone. `unstated` now answers both
callers: `statement_not_composed` refuses where nothing takes the objects, `statement_handed_on` records a
`report::Unstated` (site, step, sentence) where the decision proceeds, printed as `not stated: …` and as JSON's
`unstated`. The six composition-failure sentences end "so no statement page is composed", true at both kinds of site,
and `NOTHING_LISTED` is the handed-on site's sentence for a page that lists none of its objects (ADR 2591).

**(b) A fenced narrower rewrite gives way.** In `Answer::OrStated`, where both remedies refuse and the narrower one's
refusal is `Because::TheFence`, the stated rewrite's refusal is shown (`narrower_or_stated`, ADR 2590).

**(c) `derive:<tool>:append` is a step.** `config/steps.rs` accepts it in a `try` list and names its placement;
`DERIVED_PAGES_UNBUILT` is gone, since `archive/derived.rs::import_pages` builds the stage (ADR 2539).

**The premise that did not hold.** "Four rows' reasons change" at (b): four rows are `OrStated`, but only
`fonts/no-notdef-glyph-shown`'s narrower rewrite refuses with the fence, so only its sentences move (ADR 2590 names the
documents). "The census moves no count" held.

**Plants.** (b): with the `TheFence` rule taken out, `a_font_both_remedies_refuse_states_the_cid_remedy_s_reason` and
`a_width_no_advance_can_hold_is_refused_and_no_file_is_written` fail; restored, both pass. (a): with the push into
`Conversion::unstated` taken out, `a_halftone_statement_that_cannot_be_composed_says_why_beside_the_discard` fails.
(c): `a_derived_file_appended_as_pages_is_the_row_its_keys_make`.

**Gates.** In the worktree: `cargo test -p pdf-transform --test archive --test profiles` exit 0 (501 passed, 6
ignored; 13 passed) after the last refactor; `cargo test -p pdf-transform --lib steps` 11 passed, exit 0; clippy
`--no-deps -D warnings -p pdf-transform --all-targets` exit 101 on a sibling's two `doc_markdown` lines in
`src/archive/fonts/cid_notdef_glyph.rs`, then exit 0 once that round fixed them; `rustfmt --check` on my files and
`cargo fmt --check -p pdf-transform` exit 0. In the export: the census without the pack and with a copy of the CJK
pack exit 0 each (2 334 s for the pack run); against HEAD's, 56 changed lines in each, all
`fonts/no-notdef-glyph-shown` sentences and their document lists, identical with and without the pack, and all 12
summary lines equal; no `not stated:` line. `rank.py` exit 0 (260 lines). No output is freed, so veraPDF had nothing
to run on. `--bin cited` exit 0; `--bin quotations` exit 0; `cargo test -p conformance --no-fail-fast` 69 binaries,
470 passed, 0 failed, exit 0.
