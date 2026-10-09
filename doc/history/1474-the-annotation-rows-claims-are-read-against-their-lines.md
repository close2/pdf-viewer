# 1474 — The annotation rows' claims are read against their lines, and an ending keeps its line

Slot 1 of batch seventy-five, 2026-10-09, a clause round. ADR 1784; no row moved, no question.
**Coverage on `implemented` rows**, the 24 under `12.5`, because no `partial` row can be worked:
every `partial` leaf waits on the owner (Q308, Q271, Q348, A66's trigger, a policy syntax no
signature names, §7.4.7's patch), and §12.5's own, §12.5.6.6, waits on Q308.

**Premise.** Held in its figures: 71 claims by
`\b(is|are) (read|applied|executed|drawn|honoured|obeyed)\b`. The 24 is the population; 21 of those
rows carry the claims, and `popup.rs` is named 7 times, not 5. **Hypothesis** held in kind: the false
claims were reach claims. `/RD` was "read" by an `appearance::insets` that does not exist, and three
readers have no caller outside the tests.

**Found** (ADR 1784 §3). 65 of the 71 claims hold at a named line. Thirteen sentences were stale or
false and are corrected. The aggregates §12.5 and §12.5.6 named §12.5.6.2 as owing, which round 1476
also handed over. §12.5.6.2 and §12.5.6.14 called the window's rich text formatting owed (ADR 1642
built it), and a free text border reads no `/C`. §12.5.6.5 cited the launch and URI rows' old
statuses, §12.5.6.20 §14.11.3's, and §12.5.6.19 eight `Owed` cases where there are fourteen.

**Two defects.** A `/LE` naming a style outside Table 179 erased the whole line or polyline, against
ADR 0106 and the code's own doc comment. The slot now stays empty and the name is reported beside the
drawn line. A polygon's `/LE` was reported, and could refuse the polygon, though Table 181 calls the
entry "meaningful only for polyline annotations". It is now neither read nor reported. Each new test
fails with the old reading planted back.

**Gates.** `rustfmt --check --edition 2024` on `appearance.rs` and `tests/annotations.rs`: exit 0.
`RUSTFLAGS="-D warnings" cargo clippy -p pdf-model --all-targets`: exit 0. `cargo nextest run -p
pdf-model`: 2016 passed, 19 skipped, exit 0 (a mid-round run failed `hostile_budgets`'s Type 3 test
while slot 5 was editing `content/`). `cargo test -p conformance`: 428 passed, exit 0 (a mid-round run
failed slot 4's in-progress `bounded.rs` test). Tier 2: `raster_golden` as a `--tree 6` walk (363 s
queued, 88 s held), exit 101. Held 973, moved 1: `ContentStreamCycleType3insideType3.pdf`, slot 5's
Type 3 change, in a file that states no annotation. The six arms were not run. Every other first page
keeps its display-list and report digests, so no lane can draw this change differently there.

**Unfinished.** No host shows `annotation_state::states`, `appearance::intent` or
`measurement::annotation_measurement`, and no clause asks for one. `viewer-ui`'s note window splits on
`'\r'` and `'\n'` separately, so a CR LF pair draws an empty line (`chrome.rs` 3076, `Wrapped::new`).
Files: `crates/pdf-model/src/appearance.rs`, `crates/pdf-model/tests/annotations.rs` (two tests
rewritten), `doc/conformance/ledger.toml` (12.5, 12.5.6, 12.5.6.2, .3, .5, .7, .9, .11, .14, .19, .20),
`doc/adr/1784-*.md`, this record.
