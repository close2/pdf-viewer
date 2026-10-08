# 1424 — A face the document embeds is kerned by its own pairs

Slot 5 of batch sixty-six, 2026-10-08, a clause round and the batch's `partial`-row work. ADR 1682;
ADR 1683 not used. §12.7.4.3, §12.7.5.3 and §12.5.6.6 stay `partial`, now only for §9.6.2.2's
fourteen (Q308), with their notes moved. No question written.

**Premise.** It held: `style.rs` noted `kerning-mode:pair` and nothing read a face's pairs, and
`pdf-font` carried `GPOS` and `kern` without reading either. It did not hold for the brief's second
item: `doc/todo/22` says it is done and lists nothing owed, so there was no edge to take.

**The reader.** `pdf_font::pairs` reads `GPOS`'s `kern` feature (PairPos formats 1 and 2, with
type 9's extension) before the legacy `kern` table (formats 0 and 2, sums and override), through
`read-fonts`. `LoadedFont::pairs` refuses a substituted face and a program that is not an sfnt.
Each refusal is a phrase in the report. The work is capped at 512 pair subtables.

**The layout.** `kerning-mode` is now a style (`Kerning`). The layout kerns a run within one face:
`XAdvance` is the room after a glyph in logical order and `XPlacement` moves the glyph alone. Both
are written as §9.4.3's `TJ` numbers and measured by §9.4.4's one formula. A comb takes no kerning.
A line's last glyph, and the last one before trimmed spaces, keeps none. Vertical values and
minimum or cross-stream `kern` subtables are said, not applied. The parse still notes the property,
so a popup window, which sets no glyphs, keeps saying it.

**Evidence.** The fixtures are Liberation Sans stripped of its own tables, with one known table
written in through `pdf_font::embedding::with_tables`: ten unit tests and two `rich_text.rs`
tests. `AVAV` with `A V` at −300/2048 writes `[(A) 146.48438 (VA) 146.48438 (V)] TJ` and is
14 px narrower at 4× (3.5 pt). The page was looked at: `A V` is tight and `V A` is unchanged.
Planting "never consume the second glyph" failed its test. The writer's right-to-left side has a
unit test. The corpus was not censused for the property. `raster_golden` moved 0 of 974.

**Unfinished.** The fourteen wait on Q308. Two choices are ADR 1682's to revisit if a document
needs it: no script selection inside `GPOS`, and only pair lookups.

**Gates.** rustfmt `--check` on the nine touched sources: exit 0. `RUSTFLAGS="-D warnings" cargo
clippy --all-targets` for `-p pdf-font` and for `-p pdf-model`: exit 0 each. `cargo nextest run -p
pdf-font`: 270 passed. `-p pdf-model`: 1982 passed. rustdoc on both crates: no finding in a
touched hunk. `cargo test -p conformance --no-fail-fast`: 53 binaries, 404 passed and 1 failed.
The failure is `names.rs`, on `raster-gpu/src/encode/meet/helpers.rs:11`, a sibling's hunk in
progress; `the_frontier_map` passed. Behind the lock in one call: `raster_golden` exit 0, held
974 and moved 0; `pdf-model --test corpus` exit 0, ratchet held; `save_round_trip` exit 0,
ratchet held. That call held 87 s, peaked at 3.25 GiB and waited 1 095 s.
