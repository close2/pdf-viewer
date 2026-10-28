# 1835 — A face with no widths converts behind `layout-from-this-viewer`

Slot 4 of batch one hundred and twenty-four, 2026-10-27, an archive slot (Sonnet, high), finished from a
handover; ADRs 2504 and 2505 used, Q701 unused. I worked in `scratchpad/r1835/`. No ledger status moves
(ISO 19005 has no ledger rows).
**Premise** held: A687 (option (b) of Q687) lifts `described::describe`'s refusal of a simple font outside
the standard 14 with neither descriptor nor `/Widths`, behind a loss `as-if-printed` discards and
`as-shown` stops on, with a `/BaseFont` supplied where the file names none. **Hypothesis** (round 1821's
opening of `describe`, a loss gate, one `/BaseFont` write) held, plus a fix of the report's route
sentence, which said the face's advances "are the widths the file states" of a font that stated none.
**Built (ADR 2504).** `Loss::LayoutFromThisViewer` in the font site's conditional set (so the profile's
`discard` or `stop` decides, no TOML key); `layout_of` and `Layout` in `described.rs` with three named
refusals kept (subtype outside Type1/MMType1/TrueType, `/Widths` without a descriptor, `/BaseFont` not a
name); `/FontName` is the file's `/BaseFont` bytes; the `xmpMM:History` clause and the report line name
the loss. **ADR 2505**: the name supplied where none is stated is the embedded program's own PostScript name
(`LiberationSans`, §9.6.3 and Table 109), a choice only where a program states none; the two of the ten that stay refused (`/Type7`,
`/GontDescriptor`) are named, and the owner's stance on an appended page recorded, not built.
**Proof.** Census (pack copy, digests held), converted/conforming/refused/unreadable, HEAD → mine,
`as-if-printed` 2b pdf.js 804/2/149/11 → 809/2/144/11, corpora 351/6/116/16 → 354/6/113/16; part 4
807/0/148/11 → 812/0/143/11, 353/0/121/16 → 356/0/118/16; `as-shown` unchanged at all four; without the pack 793 → 798 and 351 → 354 (2b), 796 → 801 and 353 → 356
(4): eight documents, the ones A687 expected. veraPDF 1.30.3 passed 16 of 16 outputs (8 documents x 2b and 4),
including `T02-04-01_001_resources-basefont-missing`; every output re-renders. Twelve tests in
`tests/archive/layout_from_this_viewer.rs`; planting the defects: `/BaseFont` write disabled, one test
red; the loss left unrecorded, five red; both restored.
**Found, not fixed**: `issue17929` (descriptor under `/GontDescriptor`, with its program) and
`T02-04-01_006` (`/Type7`) stay refused; the History's closing sentence "The glyph widths the font
dictionaries state were not changed" is true of these fonts only because they stated none; a font
stating `/Widths` without a descriptor stays refused.
**Gates.** In an export of HEAD and my hunks (habit 138; siblings' hunks left out): `cargo fmt --check -p
pdf-transform` exit 0; `RUSTFLAGS="-D warnings" cargo clippy -p pdf-transform --all-targets` exit 0 (the
first run named `describe` at 101 lines and `a_metadata_property_...` at 101; the entries moved to
`restated_entries`, the literal to `..Authorisations::default()`); rustdoc `-D warnings
--document-private-items` exit 0; `cargo test --profile gates -p pdf-transform --no-fail-fast` exit 0,
763 passed, 0 failed; `--test archive` 373 passed; `archive_corpus --ignored` with the pack exit 0, 7
passed; `cross_check --ignored` exit 0, 1 passed; `tools/state.sh --round 1835 archive` exit 0; `cargo run -p
conformance --bin cited` and `--bin quotations` exit 0, no line of mine among their reds; `cargo tree -i
pdf-transform --edges all` exit 0 and reaches no render crate, so against HEAD's six arms nothing
moves; `cargo test -p conformance --no-fail-fast` 469 passed, 1 failed (sibling 1834's record names no exit status; none of mine); the validator's principle-5 finding (the supplied name was not the program's own) was fixed by me and re-gated (13 tests in the export, clippy, fmt, 16 of 16 veraPDF from a rebuilt binary pair); 6500 s for the continuation, 105 tool uses in it, 95 before. **Validation** (the orchestrator): the Opus validator's verdict was FAIL on a principle-5 defect — the supplied name recorded as a silence of the standard where Table 109 and §9.6.3 name the program's own — and on two wordings, and PASS on the loss, the refusals, the plants, the census and veraPDF; the round fixed them and the output now carries the program's PostScript name.
