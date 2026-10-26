# 1786 — `Interpreter`'s operator loop is continued in ten modules by concern

Session 1786, slot 8 of batch 118, the standing Sonnet 5.5 code-quality slot; ADR 2406; ADR 2407 unused; no question.
No bug fixed; no model change; ledger statuses unchanged (three rows name the new files beside `run.rs`).

**Choice.** `tools/state.sh gods` named the `Interpreter` impl block in `crates/pdf-model/src/content/run.rs` first (1 579
lines, in a file of 2 241), ahead of `App` in `typing.rs`; it was taken. `run.rs` keeps the loop (`run`, `run_source`,
`run_from`, `settle_step`, `run_operator`, the stream's end) and fell to 532 lines, of which 25 are the module map; the
children are `steps`, `operands`, `machine`, `graphics`, `text_ops`, `xobject_ops`, `marked_ops`, `compat_type3_ops`,
`nested` and `notes`. The hypothesis of four to six modules was exceeded (ten) and the root is not under 500 by 32 lines.

**Proof.** Token multiset of the non-comment lines, visibility qualifiers and `use`/`mod` lines removed, HEAD's file
against the root and its ten children (`proof.py`): nothing lost; the additions are seven `impl Interpreter<'_> {` heads
with their closers. Visibility was widened by the compiler's E0624 (seventeen methods to `pub(super)`); items other
modules of `content` reached keep that reach as `pub(in crate::content)`. Two intra-doc links were re-pointed.
Teaching diagrams `how-text-is-selected-and-copied` and `how-a-page-lands-on-the-screen` name moved methods and were
re-pointed; `doc/todo/74` section 20 and three ledger rows name the modules.

**Gates.** `cargo fmt --check -p pdf-model` exit 0; `RUSTFLAGS="-D warnings" cargo clippy -p pdf-model --all-targets`
exit 0; `RUSTDOCFLAGS="-D warnings" cargo doc -p pdf-model --no-deps --document-private-items` exit 0; `cargo test -p
pdf-model --no-fail-fast` exit 0 (2 215 passed, 0 failed). The move holds every token, so no pixel can move and the six
arms, the drive and the launch gates were not re-run against HEAD's six arms. `cargo test -p conformance --no-fail-fast`
exit 101: one red, `every_citation_names_a_clause_that_exists`, on `crates/pdf-model/src/media.rs:954` ("a `§` after RFC
2045"), a file this round did not touch (not mine).

**Found, not fixed.** `crates/pdf-model/src/media.rs:954` writes a `§` after RFC 2045, which the citation gate refuses.
`Machine` has eleven `pub(super)` fields read by the families; a family-local view would narrow them.
