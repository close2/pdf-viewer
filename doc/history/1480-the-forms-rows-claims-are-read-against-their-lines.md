# 1480 — The forms rows' claims are read against their lines, and the latest statement about a value stands

Slot 1 of batch seventy-six, 2026-10-09, a clause round. ADR 1796; no row moved, no question.
**Coverage on `implemented` rows**, the 30 under `12.7`: every `partial` leaf waits on the owner
(Q308, Q271, Q348, A66's trigger, a policy syntax no signature names, §7.4.7's patch).

**Premise.** Held in its figures: 30 rows, and 63 claims in 19 of them by
`\b(is|are) (read|applied|executed|drawn|honoured|obeyed)\b`. **Hypothesis** held for the sentences:
they were reach claims and stale statuses, and §12.7.5.4's and §12.6.3's sentences survived ADR 1786.
It did not hold for the defects, behind *performed* and *replace*, true only where nobody typed.

**Found** (ADR 1796 §2). 61 of the 63 claims hold at a named line. §12.7.8.3.2's "[b]oth halves …
are executed" did not hold after typing. Fourteen false or stale sentences were corrected, in 12.7.5.2.2,
.5.2.3 (two), .5.4, .5.5, 12.7.6, .6.2, .6.3, .6.4, 12.7.8 (two), .8.3.2 (two) and .8.3.4; three more
were written for the fixes and ADR 1786.

**Three defects fixed.** A reset or an import after a person typed left the typed value drawn, saved
and submitted. `forget_statements` now makes each statement the later one. A save wrote no reset, so
the file reopened showing the old `/V`; `write_resets` writes the default where it changed one. Table
240 bit 10 was owed because "ECMAScript is excluded"; a field's one-call `AFDate_*` script is that
ECMAScript, and `canonical_dates` writes §7.9.4's `D:` form. Each new test fails with the old reading
planted back. Typed, reset, imported and a saved reset reopened were rendered and looked at.

**Gates.** `rustfmt --check --edition 2024` on the eight files: exit 0. `RUSTFLAGS="-D warnings" cargo
clippy -p pdf-model --all-targets`: exit 0, and `-p viewer-core`: exit 0. `cargo nextest run -p
pdf-model`: 2027 passed, 19 skipped, exit 0. `-p viewer-core`: 374 passed, 2 skipped, exit 0. `cargo
test -p conformance`: 430 passed, exit 0 (a mid-round run failed on slot 5's knob at
`pdf-vfs/src/serve.rs:185`, which named its round). Tier 2 was not run: no drawing path changed. The
fixes change which statement a widget answers with only after a person's edit, and no corpus page is
drawn after one.

**Unfinished.** An undo after an FDF import drops its placed annotations: an import is no entry of
`viewer_core`'s log, and a `Done::Import` there is the fix. An FDF field stating no `/V` empties its
field, a reading to take to §7.3.7. **Crossed lines:** `view.rs` was edited beyond a sentence, since
both defects live there; slot 3's `viewer-core/tests/headless.rs` took a targeted edit at 1638–1702,
since it pinned bit 10's ended refusal.
Files: `crates/pdf-model/src/{view,submission}.rs`, `crates/pdf-model/src/aform/{mod,date}.rs`,
`crates/pdf-model/tests/{value_statements,canonical_dates}.rs` (new), `crates/pdf-model/tests/submission.rs`
(two tests rewritten), `crates/viewer-core/tests/headless.rs`, `doc/conformance/ledger.toml`,
`doc/adr/1796-*.md`, this record.
