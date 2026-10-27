# 1808 — XFA's fifth round: the null test runs where the form asks before a save

Session 1808, a clause round in batch 121; 2026-10-27. ADRs 2450, 2451; Q674 unused.

**Contract.** Slot 3, at A633's pace: the `validate` null test and its report (RFC 0011 section 5
step 4), and chapter 15 on a `config` `transform`'s default bind picture.

**Premises.** *A check beside the format test* held in part: the same element and value, but page
396 runs the null test only at a pre-event, page 1340's `validate` names which, and page 1340 saves
whatever validation finds, so the emptied value is the data's as the `/V`'s and the person is told.
*The unkept report is to build* did not hold: ADR 2396 built it on ADR 1592's channel; it lacked the
designer's message. *The transform's default applies* held: pages 123–124 load all data through one
loader. *The four hybrids unchanged* held: each states `acrobat/validate=preSubmit`, no `validate`
element and a `data` of `adjustData` and empty XSLT URIs (a Sonnet subagent's census).

**Built.** `xfa::config` reads every `common`'s `data` (each `transform`'s picture; page 509's
other rules name each changed field in `xfa_unkept`), `validate` and `validationMessaging`.
`xfa::validate` runs the null test where `preSave` is listed, on the `/V` the save leaves; the
template reads `nullTest` and `message`; `Written::xfa_invalid` is reported by `viewer-core`; the
format test's sentence carries the form's message; the `xfa_form` note says what is carried out.

**Files.** New: `crates/pdf-model/src/xfa/{config,validate}.rs`, ADRs 2450, 2451, this record.
Edited: `crates/pdf-model/src/{xfa.rs,view.rs}`, `xfa/{datasets,template}.rs`, `view/writing.rs`
(`-U0`), `tests/xfa_datasets.rs`, `crates/viewer-core/src/{notes.rs,viewer/editing.rs}`, RFC 0011
section 13, ledger K and K.2 (lists, note; no move), `doc/state-of-play.md`. I worked in
`scratchpad/r1808/`.

**Gates.** A sibling's `appearance.rs` broke the worktree's `pdf-model`, so tier 1 ran in my export
(habit 138), with `writing.rs` as HEAD plus my own hunks. Results: `rustfmt --check` on my files,
exit 0; `cargo fmt --check -p pdf-model` and `-p viewer-core`, exit 0 each; clippy `-D warnings
--all-targets` on both, exit 0 each; `cargo nextest run -p pdf-model`, exit 0, 2 284 passed;
`-p viewer-core`, exit 0, 424 passed; rustdoc `-D warnings -p pdf-model`, exit 0. Unit tests
`xfa::`, 33 passed. `--test xfa_datasets`, 11 passed. Planting out the `preSave` condition, the
default picture and the refusal fails the three new tests (trap 13); the plant was reversed to my
pre-plant copy, byte-identical to the worktree's. In the export: `save_round_trip`, exit 0;
`writer_corpus`, exit 0; rustdoc `-D warnings --document-private-items -p pdf-model -p viewer-core`,
exit 0 (the workspace `docs` walk stopped while queued). `cargo test -p conformance --no-fail-fast`,
exit 101, 467 passed, 2 red and not mine (`names`: a sibling's `readings.rs`; `records`: 1813's).

**Owed.** Script and datatype tests, validation before a submit, chapter 15's other rules; step 5.
