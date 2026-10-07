# 1399 — Rich text is laid out in the formatting it states, and a changed rich value regenerates whole

Rich text slot of batch sixty-two. ADRs 1634 and 1635; no question written.

**The text.** `/home/AI/specs/XFA-3_3.pdf` (SHA-256 `a3344e7e…a01e`, checked) converted by
`tools/spec-md.py` to the ignored `doc/md/XFA-3_3.md` (1584 pages, content order);
`doc/third-party-data.md` gains its section: preface page ix paraphrased, cited by section and page,
never quoted, fixtures of the tests' own words.
**The build** (ADR 1634). `pdf_model::rich_text`: `style` (CSS2 and XFA declarations, `/DS`
beneath markup, CSS2's unit rule over chapter 2's inch default), `markup` (the XHTML subset, the
`xfa:spec` list gate, an unrecognised element ignored with its content), `layout` (each run in a
face chosen `/DA` → `/DR` by Table 120 → §9.6.2.2's fourteen → §9.6.3 stand-in; the plain layout's
positions; decorations, lists, justify, margins, indent, line height, rise, spacing, scale).
`variable_text` gains `Face`, three reports, `LaidOut::fonts`; `appearance` routes a bit-26 field
and a free text note through `rich_laid_out`, comb, right-to-left and host questions one-style.
**What wins and what regenerates** (ADR 1635). The brief's premise was that §12.7.4.3 says which of
`/RV` and `/V` wins; it does not — §12.7.5.3 and Table 231 bit 26 do, and §12.5.6.2 for `/RC` and
`/Contents`: the rich entry where its characters are the plain one's, the plain one in `/DS`
otherwise, said. A rich field's changed value rebuilds the whole appearance in the stored `/BBox`;
the save writes `/RV` beside `/V` (one call in `view.rs`, round 1395's file), a clear removes it.
**Rows.** No status moves: all five stay `partial`, each note re-stated on what is left —
§12.7.4.3 and §12.7.5.3 the one-style cases and `Owed::RichTextUnapplied`'s properties,
§12.7.8.3.2 the import's `/RV`, §12.5.6.6 §12.7.4.3's residue, §12.5.6.2 the popup window's
formatting (a host's). `doc/todo/65` bucket 4, `doc/todo/22`, `doc/todo/01`, `state-of-play`.
**Pages moved: none.** `raster_golden` held 974 of 974 first pages: every witness carries its own
`/AP`, so the construction is reached by an edit, not a render (ADR 1122's finding, held).
**Next round takes:** the runs into comb cells, UAX #9 order across runs and the caret, point,
range and glyph answers; `/RV` through `FieldValue::Imported` and XFDF `<value-richtext>`; the
popup's runs in the three windows; tab stops, pair kerning, `font-stretch`, link regions.
Comment hunks: `form.rs`, `forms_data.rs`, `popup.rs`, `appearance.rs`, `xfdf.rs`,
`examples/markup_text_census.rs`. Outside my list: `lib.rs` (one `mod` line), `tests/annotations.rs`
(the `/DS` test now expects it applied), `view.rs` (the call above).

**Gates.** `rustfmt --check --edition 2024` on my 14 `.rs` files: exit 0. Clippy `-D warnings
--all-targets` for `pdf-model`: exit 0 in a private worktree at HEAD (the shared tree stops on a
sibling's `pdf-render/src/shading.rs:878`). `cargo nextest run -p pdf-model`: exit 0, 1940 passed;
`--test rich_text` then 31 passed. `cargo test -p conformance --no-fail-fast`: exit 0, 396 passed.
Behind the lock, `pdf-sandbox --bins` rebuilt first each time: `raster_golden` exit 0, held 974,
moved 0; `corpus` exit 0, every ratchet at its ceiling; `script_corpus` exit 0, 347 held; `pdf-transform
--test gate` exit 0. Tasks of user AI at the first heavy run: 140 (132 at the conversion).
