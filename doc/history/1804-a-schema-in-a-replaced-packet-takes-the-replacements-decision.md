# 1804 — A schema in a replaced packet takes the replacement's decision

Slot 7 of batch one hundred and twenty, 2026-10-27, the archive slot; ADR 2442; ADR 2443 and Q670
unused. I worked in `scratchpad/r1804/`. No ledger status moves (ISO 19005 has no ledger rows).
**Ranked** (HEAD, real producers' files, both profiles at 2b and 4, documents per reason): first
`image-smoothing` (43, both profiles' `stop`) and the `as-shown` font losses (94 over four
sentences), the fence rows (41, 29, 23, 17), part 4's `DeviceCMYK` beside an RGB intent (29, a
press profile's supply), a font no shipped face has a glyph for (16), a CJK composite font with no
pack installed (13, the census runs without one), then `metadata/extension-schemas-embedded` (12
at 2b), every one refused with `NOT_ASKED_FOR`, the placeholder its own comment says is never
reported — the largest a remedy carries under the profiles' existing losses.
**Built (ADR 2442).** Each of the twelve holds one packet section 6.6.2.1 replaces; the fresh
packet states no schema, so the container's editor found nothing and refused. It now skips a
replaced stream and names it (`SchemasDescribed::replaced`, from the row's own findings), and
`schemas_met_by_the_replacement` gives the row the packet row's own decision — the same rewrite and
loss or `preserve`, the stopping one where the packet rows differ; a replaced packet no decided row
asks for refuses with `REPLACEMENT_NOT_DECIDED`; the catalog packet is read through the same
expression's helper, `document_packet`. **Premise** held: the placeholder reaches the report.
**Hypothesis** (twenty or more, half convert) did not hold: twelve, seven convert.
**Census** (converted/conforming/refused/unreadable), HEAD → this: pdf.js 2b `as-if-printed`
789/2/165/11 → 792/2/162/11, `as-shown` 764/2/190/11 → 767/2/187/11; `doc/corpora` 2b
`as-if-printed` 348/6/120/16 → 352/6/116/16, `as-shown` unchanged; part 4 unchanged in all four.
Leaving: pdf.js's three `annotation-*-widget` files (both profiles), the four PDFBox merge
fixtures (`as-if-printed`; `as-shown` stops on a font loss); none joins; five keep other refusals.
**veraPDF 1.30.3**: 20 outputs (the seven, each profile it converts under, 2b and 4), 20 PASS.
**Drawn**: every output page against its source at scale 2, 28 pages, every difference one level
(DeviceGray 98 drawn 99 in red through the output intent); the part 4 outputs are HEAD's byte for
byte but for `stEvt:when`, so the level is the intent's. Looked at: the widgets page and the
appended page with the producer's packet. **Proof.** `tests/archive/replaced_schemas.rs`, two
tests; the call planted out fails both.
**Gates.** In an export of HEAD and my hunks: `rustfmt --check --edition 2024` on my files exit 0;
`cargo fmt --check -p pdf-transform` exit 0; `RUSTFLAGS="-D warnings" cargo clippy -p pdf-transform
--all-targets` exit 0; rustdoc `-D warnings --document-private-items` exit 0; nextest pdf-transform
708 passed, 0 failed. Census `--tree 6` exit 0 before and after; `archive_corpus --ignored` exit 0,
7 passed; `cross_check --ignored` exit 0, 13 560 compared, 0 disagreements; `tools/state.sh --round
1804 archive` exit 0 (validator missed 0, over 0 at six targets; every conforming input still
conforming). The arms: `cargo tree -i pdf-transform` reaches no crate of the corpus binary, so
against HEAD's six arms nothing can move. `cargo test -p conformance --no-fail-fast` exit 101, its
one red `every_record_since_the_rule_states_its_gates` over record 1802, not mine.
6278 s.
