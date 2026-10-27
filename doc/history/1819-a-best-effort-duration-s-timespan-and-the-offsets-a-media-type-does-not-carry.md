# 1819 — A best-effort duration's timespan and the offsets a media type does not carry

Session 1819, a clause round in batch 122 (Slot 4, Sonnet at high effort); 2026-10-27. ADRs 2472 and 2473; Q685 unused.

**Contract.** §13.2.6.3 and §13.2.6.2 finished, §13.2.6 and §13.2 derived; §13.2.5's media-duration `/T` judged.

**Premises.** *The duration's `/T` is one call of `read_offset`'s timespan reading* held, once the reading was
lifted into `read_timespan` (Table 300 once, shared by a Table 297 offset and a Table 292 duration). *Table 292
and Table 300 say the same thing* did not hold and is the finding: Table 300 says "The rendition is
non-viable"; Table 292 says the media duration *dictionary* is non-viable. A `BE` sets the second aside and
not the first, so only a `/D` of `/S /T` whose timespan `/S` is unrecognised moves the rendition (ADR 2472).
*§13.2.6.2's second sentence leaves the row owed* did not hold: "Different media types support different types of
offsets" has no `shall`, and its consequence is §13.2.4.3 NOTE 2's, which `Walk::bound` already executes for a
sound's frames, a container the reader does not read, a cut file, a URL and an outside file (ADR 2473).

**Moved.** §13.2.6.3, §13.2.6.2, and by hand the aggregates §13.2.6 and §13.2, `partial` → `implemented`;
`doc/todo/65` loses both aggregates from its list and the bucket 4 bullet.

**Built.** `media.rs`: `read_timespan` and `Timespan`, which `read_offset` now calls, and
`unrecognised_duration`, called from `media_rendition`. Two tests in `media_viability.rs` (the `BE`
timespan with eleven controls and the `MH` case; the offsets a media type does not carry, with controls).

**Files.** New: ADRs 2472, 2473, this record. Edited: `crates/pdf-model/src/media.rs`,
`crates/pdf-model/tests/media_viability.rs`, `doc/conformance/ledger.toml` (rows §13.2, §13.2.5, §13.2.6,
§13.2.6.2, §13.2.6.3), `doc/todo/65-the-remaining-frontier.md`, `doc/state-of-play.md`. I worked in
`scratchpad/r1819/`.

**Gates.** `rustfmt --edition 2024 --check` on my two files, exit 0; `RUSTFLAGS="-D warnings" cargo clippy -p
pdf-model --all-targets`, exit 0 (an earlier run was red on a sibling's `xfa/validate.rs` doc and went green
unchanged by me); `cargo test -p pdf-model --lib media::` 39 passed, `--test media_viability` 20 passed (18
before), `cargo test -p viewer-core --test renditions` 5 passed, all exit 0; rustdoc `-D warnings` on
`pdf-model`, exit 0. Plants (trap 13), each reversed by its own patch: dropping the `unrecognised_duration`
call fails the first new test; replacing the frame count's refusal with `unwrap_or(0)` fails the second and
`an_end_past_the_clips_own_length…`. `--bin cited`, `--bin inapplicable`, `--bin quotations`, exit 0 (no
diverging quotation on a §13.2 row). `cargo test -p conformance --no-fail-fast`, exit 101, 467 passed, three
reds in `--test conformance`, none mine: `every_quotation_is_the_standards_own_words` (`colour_transform.rs`,
`bystanders.rs`; my `media.rs:1139` was red once and is fixed), `every_table_designation…` (`pdf-jpx-write`),
`the_ledger_agrees…` (§12.5.6.23's and §K.2's test names). The Opus validator's one finding (a URL case the test's comment named and its body lacked) I fixed. Nothing drawn, no dependency added.

**Found, not fixed.** A negative `BE` duration is set aside, as Table 291 allows, with no reason of its own. **Validation** (the orchestrator): the Opus validator's verdict was FAIL on one test comment naming a case the test lacks and PASS on the code, the tests, the plants and the clause reading; the round fixed the finding.
