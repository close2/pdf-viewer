# 1718 — Review 1704's small findings, each with its clause

Session 1718, slot 7 of batch one hundred and ten, a clause round; 2026-10-23. No ADR (2270, 2271
unused), no `Q`, no ledger row moved; every `.rs` edit is a comment but the rename and narrowing.

**Citations.** Each read against `doc/md/` before it was written (habit 111), quotations verbatim:
`font_widths.rs` cites §9.6.2.1's Table 109 `/Widths` sentence, §9.6.3 and ISO 19005-2 §6.2.11.2
/ 19005-4 §6.2.10.2; `scripts/state.rs` §12.6.3's Tables 197 and 198 (the `/C` order quoted) and
§12.7.3's `/CO`; `scripts/commit.rs` Table 199's order and its `/C` sentence on `/CO`;
`scripts/events.rs` §12.6.3 and §12.6.4.17's Table 221; `page_contexts.rs` ISO 19005-4 §6.2.3 and
§6.2.4.3, §8.6.5.6 (quoted), §7.7.3.4, §7.8.3 (quoted), Table 50, §11.4.7, §8.6.5.5's Table 65,
Tables 31 and 401; `linked_popups.rs` ISO 19005-2/-4 §6.3.2, Tables 167, 171, 172, 175 and 186.
Its test file's seven tests each say where the expected value comes from (Table 167's flag values,
§12.5.2's one-page rule, Table 186's `/Open` default, the fixture's own policy).

**Code.** `ConversionContext::to_owned` is `to_conversion` (it shadowed `ToOwned::to_owned` with
another return type); `Conversion::context` is `pub(crate)`, its only readers being `pdf-colour`'s.
The fields stay `pub`, and the type now says why: `pdf-model` builds the context by name, and a
positional constructor would accept `output_intent` and `cmyk_group`, one type, swapped. The one
call outside `pdf-colour` is `colour.rs`'s `SolidColours::paint`, changed by one targeted line
(not where the type lives, so not handed over; the hunk is in the report). `fonts.rs`'s
`widths_agree_with_the_program` says the current reason (ADRs 2163, 2180). `decode_answer`'s doc
moved from `measurement.rs` to `answer.rs`. ADRs 2209 and 2233: dated amendments in the Status
line (ADR 0265) that ADR 2242 decision 2 supersedes their red-gate acceptances.

**Premises that did not hold.** `doc/todo/65` section 6 already says uncoloured tints convert at
paint (ADR 2212), rewritten at batch 109's merge; nothing to edit. A moved golden page is named by
the round's record (`doc/todo/02` section 2 rule 3; `raster_golden.rs`, "the round that moved them
says why"), not its ADR, and records 1672 and 1673 named all five, so no addendum is owed; the
brief's ADR 2181 is about `/LastChar`, the CFF move's ADR is 2180.

**Gates.** The session worked in `scratchpad/r1718/`, whose export is HEAD plus this round's hunks
only (habit 138). There `cargo fmt --check`, `RUSTFLAGS="-D warnings" cargo clippy --all-targets` and
`cargo nextest run` exit 0 for `pdf-colour`, `viewer-confined`, `pdf-archive`, `pdf-transform`,
`pdf-model` (137, 117, 288, 658, 2181 passed, `pdf-sandbox`'s `dev` worker built, trap 10). `cargo test -p conformance --no-fail-fast` in the worktree:
every binary passes (`evidence`, `adr_status`, `records`, `conformance`'s citations and quotations,
`spelled_ordinals`, `round_numbers` among them) but `names` (`viewer-ffi`) and `variables`
(`tools/zoom-probe-drive.py`), both a sibling's mid-edit. No arm or `raster_golden` owed: no pixel-reaching line
changed. Duration 2589 s.
