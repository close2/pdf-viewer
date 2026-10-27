# 1820 — XFA's sixth round: the datatype test, and the validations before a submit

Session 1820, a clause round in batch 122; 2026-10-27. ADRs 2474 and 2475; Q686 unused.

**Contract.** Slot 5, at A633's pace: RFC 0011 section 5 step 4's validations half, the datatype
test (XFA 3.3 pages 381 and 386) and the run before a §12.7.6.2 submit (page 1340's `preSubmit`,
page 447) on ADR 1592's channel. Finish line: row §K.2's note says the validations sentence is
discharged and calculations and events are the half owed.

**Premises.** *The datatype test is a check beside the null test* held, and needed the canonical
value the format test needed (page 382); the first half of `typed` became `canonical_text`. *The submit calls `validate` with its event* held, with a fact
the brief lacked: `compose` returns a `Submission`, not a report channel, and already names Table
227 bit 2's Required field on `Submission::owed`, leaving the cancelling to the host's policy (ADR
1198). The submit does the same, because page 1340's rule that an error cancels the submit must stay an *ask*.

**Built.** `xfa::validate::before(Occasion, ..)` runs the null, datatype and format tests;
`template::Digits` keeps a `decimal`'s `fracDigits` and `leadDigits`; `Config` reads `preSubmit`
(true where no `validate` element is stated); `Resource::before_submit`; `submission::compose`
runs it over every field the view shows. A save runs null and datatype; a submit adds the format
test (a save's is the write's refusal, ADR 2434). A picture or locale this reader cannot read
leaves a test *unrun* and says so; a dynamic, regenerated or unreadable form is named unvalidated.

**Files.** New: ADRs 2474, 2475, this record. Edited: `crates/pdf-model/src/{xfa.rs,submission.rs}`,
`xfa/{validate,datasets,config,template}.rs`, `tests/xfa_datasets.rs`, ledger K.2 (lists and note;
status stays `partial`), RFC 0011 section 13, `doc/state-of-play.md`, `doc/todo/65`. I worked in `scratchpad/r1820/`.

**Gates.** A sibling's `pdf-sandbox` edit broke the worktree's build, so tier 1 ran in my export
(habit 138), my files whole. There: `cargo fmt --check -p pdf-model`, exit 0; clippy `-D warnings
--all-targets` on `pdf-model` and `viewer-core`, exit 0 each; `cargo test -p pdf-model
--no-fail-fast`, exit 0, 2309 passed; `xfa::` unit tests 40 passed; `--test xfa_datasets` 12;
`--test submission` 35; `--profile gates --test save_round_trip -- --ignored`, exit 0, 1 passed;
rustdoc `-D warnings --document-private-items -p pdf-model`, exit 0; the workspace `docs` walk stopped after 1 017 s queued. Planting out the datatype test
and the `preSubmit` default fails 7 unit tests and the new integration test (trap 13); reversed,
byte-identical. `cargo run -p conformance --bin cited`, exit 0. `cargo test -p conformance
--no-fail-fast`, exit 101, 468 passed, 2 red, neither mine: the quotation gate
(`colour_transform.rs`, `bystanders.rs`) and the table-designation gate (`pdf-jpx-write`). The validator's finding, XFA text quoted verbatim in comments and ADRs, was fixed by me.

**Owed.** Calculations and events (step 5, a FormCalc engine); the script test and page 447 step
3's validation scripts; a host that reads `Submission::owed` to cancel on an error-level failure. **Validation** (the orchestrator): the Opus validator's verdict was FAIL on XFA 3.3 text quoted where it may only be cited and PASS on the code, the tests, the plants and the clause reading; the round fixed the finding.
