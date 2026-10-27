# 1798 — Annex K leaves `out-of-scope`, and a begin at the clip's start is honoured

Session 1798, a clause round in batch 120 (the ledger slot); 2026-10-27. ADRs 2430, 2431; Q664 unused.

**Contract.** Slot 1: Annex K against RFC 0011 section 11; §13.2.4.3's `/B`; two miscitations.

**Premises.** *The XFA entry meets section 11's condition* held (A639; "XFA's static half is in
scope"). *K.1 and K derive* did not: K.1 is a leaf and moves on its own reading. *The `/B` keeps
§13.2.4.3 `partial`* held in part: an `/E` past the clip's end and a missing marker keep it so.
*The entry admits the offset* held: "§13.2.2's viability computed and reported in the clause's own
terms" is in scope, and reading a dictionary decodes nothing. The miscitations held: Table 400 is
§14.11.4's, Table 126 a spot-function table; §9.9 prints Table 124.

**Moved.** §K.1 `out-of-scope` → `implemented`: "If an interactive processor supports XFA forms,
that processor shall clearly indicate to the user when they are interacting with an XFA form that
they are interacting with an XFA form", its permission taken for page generation. §K.2
`out-of-scope` → `partial`: three writer's rules kept, "all form-related events such as
calculations and validations" owed and said. K → `partial`, derived (ADR 2430). §13.2.4.3 stays
`partial`, a `/B` at the clip's start now honoured (ADR 2431).

**Built.** `notes::xfa_form` no longer says "this program does not implement XFA" of a hybrid
form (ADRs 2396, 2414 made it false); `media::is_start` reads §13.2.6.2's offset down the chain.

**Files.** New: ADRs 2430, 2431, this record. Edited: `pdf-model`'s `media.rs` and
`tests/media_viability.rs`, `viewer-core`'s `notes.rs`; a comment line in `pdf-font`'s `sfnt.rs`,
`pdf-transform`'s `archive/prepare.rs` and `tests/archive.rs`; ledger K, K.1, K.2, §13.2.4.3
(`-U0`); RFC 0011 section 11; `doc/todo/65`; `doc/state-of-play.md`. I worked in `scratchpad/r1798/`.

**Gates.** A sibling's `pdf-syntax` edit broke the shared build, so tier 1 ran in an export of HEAD
plus my hunks (habit 138): `rustfmt --check` on my six files and `cargo fmt --check` on `pdf-model`,
`viewer-core`, `pdf-font`, `pdf-transform`, exit 0; clippy `-D warnings --all-targets` on the four,
exit 0; `cargo nextest run` on the four, exit 0, 3 690 passed; rustdoc `-D warnings` on the four,
exit 0. The new test fails under each of two plants (trap 13). `--bin inapplicable`, `--bin cited`,
`--bin quotations`, exit 0 each; `tools/state.sh --round 1798 ledger`, exit 0. `cargo test -p
conformance --no-fail-fast`, exit 101, 467 passed: `records` red on records 1801, 1802 and 1805,
none mine.

**Owed.** `media.rs` now executes §13.2.6.2's and §13.2.6.3's subtype rules under `out-of-scope`
rows the seventh sweep names; they are another slot's. `xfa.rs`'s module comment still opens "This
program does not implement XFA"; it is the XFA slot's file.
