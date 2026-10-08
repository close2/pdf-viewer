# 1468 — The text rows' claims are read against their lines, and a flags word has one reading

Slot 1 of batch seventy-four, 2026-10-08, a clause round. ADR 1772; no row moved, no question.
**Coverage on `implemented` rows** — the 65 under `9.` and §10.5's note — since every `partial` leaf
waits on the owner (Q308, Q271, Q348, A66's trigger, a policy syntax no signature names).

**Premise.** Held: 65 rows and 61 claims, counted with `\b` round the pattern (70 without it, `this
read` matching `is read`); §10.5's "not cached" sentence was there. It did not hold in size: the stale
part of §10.5's note was five sentences describing ADR 0479's design, not one.

**Found.** 55 of the 61 claims hold at a named line (ADR 1772 §3). False: 9.2.4's and 9.6.4's "read by
nobody" for the Type 3 boxes (the redaction reads both, ADRs 1351, 1363); 9.7.4's "only for §9.7.5.2's
compatibility rule" (no such check; the rule is §9.7.3's and binds the file); 9.8.1's "read as inputs by
nothing" (`/FontFamily` by the rich text layout, `/CharSet` by the validator); 9.8.2's "one reader".
9.7.3's three readers are four. One cited name did not exist (`collection_meaning`), and two notes cited
`content.rs` for text in `content/run.rs` and `content/ext_gstate.rs`. Every note is corrected.

**The defect.** `rich_text::layout::family_of` read `/Flags` as an `i64` and tested its bits, so `-64`
read as bold italic and a bold run under such a `/DA` face was set in the regular face. The word is now
`pdf_font::descriptor_flags`, every bit clear outside §9.8.2's unsigned 32 bits, asked by `metrics::flag`
and `family_of`. `a_flags_word_outside_thirty_two_unsigned_bits_states_no_style` failed on the old
reader (Helvetica where `/HeBo` is owed) and passes on the new one.

**§10.5.** The note now says what ADRs 1266 and 1279 built: no colour carries the function, the transfer
channel maps the finished pixel, a shading under a transfer is cached by its resources' `/ColorSpace`
entry like any other, a named one included (ADR 1765), and a type 1 shading keeps its device program.

**Gates.** `rustfmt --check --edition 2024` on the four Rust files: exit 0. `RUSTFLAGS="-D warnings"
cargo clippy --all-targets` `-p pdf-font` and `-p pdf-model`: exit 0. `cargo nextest run -p pdf-font`:
281 passed; `-p pdf-model`: 2013 passed, 19 skipped. `cargo test -p conformance`: 427 passed, exit 0.
Tier 2 under the lock, small walks: `raster_golden` 2 passed, held 974, moved 0 (491 s queued, 53 s
held); `pdf-model --test corpus` 1 passed (3 056 s queued, 16 s held); `text_extraction` 4 passed (0 s
queued, 45 s held); each exit 0. The six arms were not run: `family_of` answers every word in 0 to 2^32
as before, and `font_flags_census` finds no corpus descriptor stating one outside it.

**Unfinished.** `pdf-archive`'s `is_symbolic` (a validator, not this round's crate) tests bit 3 itself.

Files: `crates/pdf-font/src/{metrics,lib}.rs`, `crates/pdf-model/src/rich_text/layout.rs` (`family_of`),
`crates/pdf-model/tests/rich_text.rs` (one test), `doc/conformance/ledger.toml` (9.2.4, 9.3.1, 9.6.4,
9.7.3, 9.7.4, 9.8.1, 9.8.2, 9.10.2, 10.5), `doc/adr/1772-*.md`, this record.
