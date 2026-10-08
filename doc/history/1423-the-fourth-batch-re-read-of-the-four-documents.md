# 1423 — The fourth-batch re-read of the four documents

Slot 4 of batch sixty-six, 2026-10-08, a docs round. ADR 1680. No ledger row moved; no question
written.

**Premise.** The four documents' sizes held (757, 64, 1 379 and 88 lines). The second half did not:
`CLAUDE.md`'s grep prints 230 lines and not one is a round's history. `tools/comment-history.py`
calls 18 of them history, and all 18 are a `setsid` session, a batch branch's first session or
`ledger_notes.rs` describing its own gate. The history the rule is about is spelled `round <number>`:
56 comment lines in 20 files, which neither the grep, `spelled_ordinals.rs` nor the classifier
counts. ADR 1680 section 4 maps each round to its ADR and gives the sweep to the instruments slot
first, then to each crate's owner.

**The re-read.** `cargo metadata` against the crate map's rows: all 43 members and `kio/`, nothing
missing. `tools/state.sh navigation`: one absent pointer, `doc/questions/A170`, which is the owner's
and uncommitted in the main checkout. A throwaway extractor resolved every backticked path and
identifier in the four documents against the tree. Then every sentence was read as *what is*. ADR
1680 section 2 lists the fifteen false claims and what replaced each: a type renamed
(`QuorraWindowRenderer`), six sidebar tabs where three were written, 40 ABI entry points where 35
were, modules no row named, a fourth confined program, a hand-written fuzz-target list that had
fallen behind its directory, and hayro's lead in features that are all in this tree.

**Lines lost.** `doc/state-of-play.md` 1 379 → 1 032 (21 870 → 16 307 words): `raster-gpu`'s
construction list, the converter's mitigations, the redaction's and the transform suite's operator
detail, and the file-system faces each went to the one sentence that the crate map, the mitigation
catalogue, `doc/todo/57` or an RFC already expands. Every ADR they cited is still cited. `doc/PLAN.md`
757 → 756 (9 125 → 9 115), `doc/crate-map.md` 64 → 64 (10 539 → 10 524) and `doc/HANDOVER.md` 88 → 88
(1 600 → 1 608) were corrected rather than cut. Two `viewer-ui` comments that told a Wayland defect
in the past tense (`cadence.rs`, `surface.rs`) now state the present reason under ADR 0384.

**Gates.** `cargo test -p conformance --no-fail-fast`, after the record: exit 101, 403 passed and 2
failed, both in `tests/bounded.rs` (slot 6's lanes mid-build: `batch.sh:278`'s lock without a kind,
and `clock_gates`); `records`, `documents`, `names` and `spelled_ordinals` pass. `tools/batch.sh check`: exit 1, only `cargo fmt
--all --check` on siblings' files. `--bin quotations`: no diverging quotation in the four documents.
`--bin pointers`: the one `A170` pointer. `rustfmt --check --edition 2024` on the two `viewer-ui`
files: exit 0. `cargo clippy -p viewer-ui --bin quorra`: exit 0, no warning in `viewer-ui`. With
`-- -D warnings` it fails in slot 5's `pdf-font/src/pairs.rs`. `RUSTDOCFLAGS="-D warnings" cargo doc
--workspace --no-deps`: exit 101, on HEAD's own `conformance` and `viewer-ui/src/chrome.rs` links;
`cargo rustdoc -p viewer-ui --bin quorra -- -D warnings` names no line of the two files.
