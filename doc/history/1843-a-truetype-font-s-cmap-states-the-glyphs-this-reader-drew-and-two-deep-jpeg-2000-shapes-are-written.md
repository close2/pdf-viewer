# 1843 — A TrueType font's cmap states the glyphs this reader drew and two deep JPEG 2000 shapes are written

Slot 4 of batch 125, 2026-10-28, an "archive" round (Sonnet, high), in three sessions and a fourth that
answered the validator, between about 03:00 and 08:30 by the clock (the boundaries were not timed).
ADRs 2520 (R11) and 2521 (A691) are used; Q709 is unused. The sessions worked in an export of the
tree, since the shared worktree did not compile for a sibling's private `MAX_VALUES` at the start.

**Premise.** R11 is one `cmap` subtable from the reader's code-to-glyph map: held, as two subtables,
Macintosh Roman (1, 0) and Microsoft Symbol (3, 0), with the font made the symbolic font ISO 32000-2
§9.6.5.4 describes, because a non-symbolic route needs a name for every code. A691 is two returns
removed: held, `reads_back_as` already compared the opacity depth; the "veraPDF failing" on the two
outputs reproduced only with `/SMaskInData 1`.

**Built.** `archive/transcode.rs` loses `DEEP_OPACITY_AT_ANOTHER_DEPTH` and `DEEP_GREY_WITH_OPACITY`.
`archive/fonts/truetype_cmap.rs` and `Rewrite::TrueTypeCmapRestated` answer the four `cmap` rows; the
three encoding rows (`Answer::OrCmapRestated`) take it as their second answer where their own rewrite
refuses, which is the brief's own plant (no `/ToUnicode`). The proof reloads the font: all 256 codes on the
glyph they reached. The file's `xmpMM:History` names the clause and the fonts (A48's condition), and a
packet that will not take it withdraws the rewrite. Bounds: the checker's places, 32 MiB of copied program
(`MAX_COPIED_BYTES`) with its own sentence, and no copy for a file another row refuses by name. The old
descriptor and program are not written to the output. Tests: `tests/archive/truetype_cmap.rs` and
`tests/archive.rs`. The veraPDF issue draft with
two reproducers is `doc/pdfa-blocking-issues/verapdf-jpeg2000-issue.md`, indexed from
`doc/pdfa-blocking-issues.md`, whose R11 paragraph and section 8 (the `uniXXXX` checker defect) are rewritten.

**Measured.** The 32 documents R11 names, per arm: exit 0 from 2 (part 4: 4) to 14, veraPDF at the arm's
target 13 pass and `pdf.js:issue6961` fails (a `pdf-archive` checker defect, ADR 2520 section 8). Census
with the pack, converted failing documents (pdf.js then corpora): `as-if-printed` 2b 838 to 848 and 386 to
388, part 4 841 to 849 and 385 to 387; `as-shown` 2b 797 to 807 and 333 to 335, part 4 801 to 809 and 331 to
333. R11 by `rank.py`: 30 documents to 13 under `as-if-printed` 2b. The 32-font 17 MB program: 37 s and
147 MiB, refused with the budget sentence (429 MiB and 119 s before the budget).
**Gates.** In an export: `cargo test -p pdf-transform --test archive` exit 0 (413 passed); `--lib truetype_cmap`
exit 0 (2 passed); `cargo clippy -p pdf-transform --all-targets -- -D warnings` exit 0; `rustfmt --edition 2024`
on the files touched, no change left; `--test archive_corpus -- --ignored` and `-p pdf-archive --test cross_check
-- --ignored` exit 0 (7 passed) and exit 0 (1 passed); `tools/state.sh --round 1843 archive` exit 0 on the first build; `--bin cited` and
`--bin quotations` exit 0; `cargo test -p conformance --test records --test adr_status --test evidence` exit 0.
**Found, not fixed.** `issue6961` and the leniency of `pdf-archive`'s name check. The 13 documents the three
encoding rows still refuse (reason per document not traced). R03a (the `.notdef` row) is owed. The sign-bit
finding in the veraPDF draft has no reproducer. `decision.rs`'s `REFUSED_BY_NAME` comment still calls the
neighbouring rows a fence. The restated fonts of the two documents the second answer freed were not rendered. **Validation.** An Opus validator failed the first reading on eight findings — the slot's own case refused one row later, a fence sentence users still saw, an ADR that never asked whether the operation invents marks, a false orphan cost, no byte budget on copied programs, the glyph-list reading left unsettled, no history entry, and licensed sentences quoted — and after the round's fixes accepted the behaviour and the bounds, leaving two wording items the round then made good.
