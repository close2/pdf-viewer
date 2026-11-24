# 2218 — The closing gate: every class is mapped, and every default is counted

Lane 3, the silent-repair campaign's step 3 (the owner, 2026-11-23) and fix round 4 (review 2194's
T05). ADRs 3573 to 3575.

- **The mapping gate** (`tools/conformance/tests/repair_classes.rs`, ADR 3573) parses the three
  `classes!` blocks, `pdf_archive::repairs::MAPPING` and the table's `id:` lines, because the
  conformance crate links nothing. It fails on an unmapped class, a mapping naming no class, a row
  the table lacks, a class mapped twice and a `no_row` without a reason. A planted
  `syntax/planted-unmapped` in `pdf_syntax::repair` was named; a self-test plants each condition on
  every run.
- **The census gate** (`tools/conformance/tests/repair_census.rs`, ADR 3574): every
  `unwrap_or*`/`.ok()`/`.clamp(` line in the three crates' non-test, non-writer code, answered by a
  class or recorder within twelve lines or a `// not a repair:` within six lines above. A per-file
  ratchet (`HELD`) holds the rest and fails when a count rises or falls without the table moving.
  A plant before a test module was named; one after it was not, and the limit is written down.
- Six Sonnet helpers under one brief — read every site; mark only what is no file's value or the
  specification's default, as `repair_census.rs`'s comment states; leave a repair counted — wrote
  the markers, comment lines only. The round checked the clause-citing ones
  and removed one marker (a blend mode name no table lists read as `Normal`) with no clause
  behind it. The marker reach was cut from twelve lines to six when 76 sites turned out to be
  answered by a marker written over another default.
- **T05** (ADR 3575): eight `model/…` classes logged where the interpreter makes the repair, each
  mapped to section 5.1 alone, each with an acceptance test in `pdf-model`'s `repair_classes`.
- The helpers' lists of sites read as repairs (left counted) are the next fix rounds' input, by
  crate: `pdf-syntax` `xref.rs` and the decoders' `.ok()`, `pdf-font` metrics and maps,
  `pdf-model` `appearance.rs`, `annotation.rs`, the image unpackers, `viewer_preferences.rs`.
- Habit: a census window that lets a marker answer sites it was not written for counts the
  marker's reach, not the reading; measure how many sites each extra line of reach answers.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-font -p pdf-archive -p viewer-qt -p
conformance`: `cargo fmt --check` 0; clippy `-D warnings --all-targets` 0; `cargo build -p
pdf-sandbox --bins` then nextest `--lib --bins --tests` 4801 of 4803 passed, the two failures this
record's own format (fixed, then `cargo test -p conformance` 0); rustdoc `-D warnings
--document-private-items` of the four library crates 0. No public type changed, so no fuzz check.
`tools/state.sh --round 2218 launch` 101: eight figures above their bands, six of them opens this
round's code does not reach (the channel's regression round 2215 owns); the two frames are 0.4 %
and 0.05 % over, against one comparison per operator this round added.
