# 1751 — The text object, text state and text positioning operators are three groups of Table 50

Session 1751, slot 8 of batch 114, the standing code-quality slot; ADR 2336; no question.
No bug fixed; no model change; ledger statuses unchanged.

**Choice.** The top hand-written item `tools/state.sh gods` named after round 1743 was `run_text_op` in
`crates/pdf-model/src/content/run.rs`, the longest family method of the operator loop (about 125 lines; the census's
`Interpreter` impl block is the file's one and is not split here). It is now a 20-line dispatch to
`run_text_object_op` (`BT`, `ET`), `run_text_state_op` (`Tc` `Tw` `Tz` `TL` `Tf` `Tr` `Ts`, handed the
`GraphicsState` alone) and `run_text_positioning_op` (`Td` `TD` `Tm` `T*`, an associated function: clippy's
`unused_self` showed it uses none).

**Proof.** The multiset of the file's non-comment lines against HEAD's (script `split.py`, anchors asserted, arm
bodies cut and pasted as line lists): no line is lost; the file gains four signatures, three `match` heads with
the wildcard arm and braces, two destructuring prefixes, the dispatch arms and three doc comments. No ledger row
names a line range of `run.rs`; the teaching diagram names `run_from` and `run_operator`, which did not move.

**Gates.** `cargo fmt --check -p pdf-model` and `RUSTFLAGS="-D warnings" cargo clippy -p pdf-model --all-targets`
(pedantic) exit 0; `cargo test -p pdf-model --no-fail-fast` exit 0 (259 s). Against HEAD's six arms on an export of
this round's one file: cpu, gpu and compute at 1x and 4x each exit 0, 968 pages at 1x and 965 at 4x, and no page
differs in frame digest or mean error (2 912 s under the lock). `cargo test -p conformance` `evidence`, `adr_status`, `records`, `names`, `teaching`, `ledger_notes` and `spelled_ordinals` pass; `round_numbers` fails on a sibling's example file. `raster_golden` was queued 2 931 s for the lock behind a sibling's clock hold and cancelled unrun; the six arms' digests carry it.

**Found, not fixed.** `show_text`'s setup is still about 100 lines of reasoning (round 1743). The next census
items are `Host` in `quorra-confined.rs` (1 306 lines in one impl block) and `Vfs` in `pdf-vfs`; both are hand-written.
