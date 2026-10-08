# 1430 — A run is kerned by its own script's table, and a contextual lookup is not a pair

Slot 5 of batch sixty-seven, 2026-10-08, a clause round and the batch's `partial`-row work. ADR
1696; ADR 1697 and Q326 not used. §12.7.4.3, §12.7.5.3 and §12.5.6.6 stay `partial`, still only for
§9.6.2.2's fourteen (Q308), and their notes now name this round's work.

**Premise.** It held. `pairs.rs` took every `kern` feature record by tag, every script's lookups
together, and skipped every lookup that was not type 2. `doc/history/1424` says no corpus search
was made.

**Script selection.** A run's `GPOS` lookups are now the ones OpenType's layout selects: the script
table registered for the script the run's characters resolve to, `DFLT` where none is registered,
and its default language system's features, the required one included. `Scripts.txt` and
`PropertyValueAliases.txt` (UCD 18.0.0, vendored) are compiled in by `build.rs`. `Common` and
`Inherited` take the nearest specific script. The registry's exceptions (`kana`, four padded tags,
ten "v.2" tags tried first) are checked against `data/opentype/script-tags.txt`. Two glyphs of two
scripts are no pair.

**Contextual lookups stay out.** XFA's change list defines pair kerning as kerning by the two
adjacent glyphs alone, and a `ChainContextPos` reaching a `PairPos` kerns by context. The fixture
shows `T A V` not moved where the whole feature would move it −250, and `V A` kerned.

**Corpus.** A pikepdf census decoded every stream and every `/RC`, `/RV` and `/DS` string of 1489
unique documents. None states `kerning-mode` or `kerningMode`, and a planted document was found.

**Evidence.** Seven new unit tests and one `rich_text.rs` test (`latn`'s −300 writes 146.48438,
48.828125 with no script). Four plants each failed: `DFLT` first, every lookup, a wrong Lao tag, no
split at a script change. The page was looked at: `A V` tight, `V A` unchanged, 14 px narrower at 4×.

**Unfinished.** The fourteen wait on Q308. A language-specific system is never selected (ADR 1696).

**Gates.** rustfmt `--check` on the five touched sources: exit 0. `RUSTFLAGS="-D warnings" cargo
clippy --all-targets` for `-p pdf-font` and `-p pdf-model`: exit 0 each. `cargo nextest run -p
pdf-font`: 277 passed. `-p pdf-model`: 1984 passed, 19 skipped. rustdoc: no finding in a touched
hunk. `cargo test -p conformance --no-fail-fast`: 54 binaries. The first run had 407 passed and
exit 0, `the_frontier_map` among them. The last had 407 passed and 1 failed:
`bounded.rs::every_walk_a_state_section_runs_is_locked_in_its_declared_lane`, slot 6's ADR 1698
test in progress. Behind the lock in one `--tree 6` call: `raster_golden` exit 0, held 974 and moved
0; `pdf-model --test corpus` exit 0, ratchet held; `save_round_trip` exit 0, ratchet held. It held
155 s, peaked at 3.36 GiB and waited 47 s. The census held 42 s.
