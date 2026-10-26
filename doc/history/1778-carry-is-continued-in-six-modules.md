# 1778 — `Carry` is continued in six modules by concern

Session 1778, slot 8 of batch 117, the standing Sonnet 5.5 code-quality slot; ADR 2390; ADR 2391 unused; no question.
No bug fixed; no model change; ledger statuses unchanged (nine rows name the new files beside `structure.rs`).

**Choice.** `tools/state.sh gods` named `Carry` in `crates/pdf-transform/src/structure.rs` (one impl block of 1 085
lines, 21 methods, in a file of 1 417). The methods are in `reconcile.rs`, `hierarchy.rs`, `wrapper.rs`, `keys.rs`,
`write.rs` and `report.rs`, one `impl Carry` each; `structure.rs` fell to 451 lines and keeps the types, `Host`, `plan`
and the `/K` readers. The hypothesis (four to six modules, root under 500) held.

**Proof.** Token multiset of the non-comment lines, `pub(crate) ` and `pub(super) ` stripped, `use` and `mod` lines
removed, HEAD's file against the root and its six children (`proof.py`, from `split.py` whose anchors are asserted):
nothing lost; the additions are six `impl Carry {` heads with their braces. No method was renamed; no teaching diagram
names `Carry`; `doc/todo/74` (section 19) and nine ledger rows name the modules.

**Gates.** `cargo fmt --check -p pdf-transform` exit 0; `RUSTFLAGS="-D warnings" cargo clippy -p pdf-transform
--all-targets` exit 0; `RUSTDOCFLAGS="-D warnings" cargo doc -p pdf-transform --no-deps --document-private-items`
exit 0; `cargo test -p pdf-transform --no-fail-fast` exit 0 (124, 326 passed, the ignored corpus gates aside); on an
export of this tree with its sandbox worker rebuilt: `split_corpus` exit 0 (79.39 s of test time), `merge_corpus` exit 0 (88.93 s) and `pages_corpus` exit 0 (154.11 s), each under `--profile gates`.. `cargo tree -i pdf-transform` prints `pdf-vfs`, `viewer-ffi` and `viewer-host` and their dependents; the move holds every token, so no arm's pixels can move and the six arms were not re-run. `cargo test -p conformance --no-fail-fast` exit 0 (423 s; no red).

**Found, not fixed.** `Carry::keep` and `Carry::child` each carry an `#[expect(clippy::too_many_arguments)]` for the walk's
threaded state; a small walk-state struct would remove both. `Carry::finish` is about 90 lines and builds elements,
parent tree and root in one body.
