# 1719 — The C ABI splits by concern

Session 1719, slot 8 of batch 110, the standing code-quality slot; ADR 2272; no question.
No bug fixed; no model change; ledger statuses unchanged.

**Choice.** `crates/viewer-ffi/src/abi.rs` (9 908 lines, first hand-written source file in `tools/state.sh gods`)
is the module root (88 lines) over eighteen children in `src/abi/`: `layouts`, `identity`, `commands`,
`event_stream`, `rendering`, `queries`, `conversion`, `panel_reads`, `pointer`, `forms`, `edits`, `host`,
`page_reads`, `popup_reads`, `structure_reads`, `collection_reads`, `window_calls`, `shared`. The gates that
count entry points were not an obstacle: the three tests that read `abi.rs` now read the directory.

**Proof.** The lines of `abi.rs` at HEAD against the union of the new files differ only by `use` lines,
module headers, the `pub(super)` prefix on six shared helpers and 71 intra-doc links qualified by path.
`tools/state.sh hosts` still prints 355 entry points; `unsafe_position` holds 355 attributes and 336 signatures.
Ledger row 12.3.5 re-points to `abi/collection_reads.rs`; `crate-map.md`, `todo/74` and `tools/state.sh` follow.
No teaching diagram names the file.

**Gates.** `cargo test -p viewer-ffi`: every integration test passes, the C driver among them (154 s);
the doctest step fails with `can't find crate for viewer_core`, a harness failure in `lib.rs`
that holds no abi example. Clippy `-D warnings --all-targets`, `cargo fmt --check -p viewer-ffi` and
`cargo doc -p viewer-ffi --no-deps` exit 0. `cargo test -p conformance`: the quotation and citation gates fail on sibling files
(`pdf-transform/src/archive/*`, `pdf-model/src/view/scripts/commit.rs`), none in `viewer-ffi`.

**Found, not fixed.** `tools/state.sh hosts` prints "Query: 35 of 36 reach the ABI"; the doctest failure above
is unexplained; the doc links gain long lines (`[`x`](super::x)`) that a later pass may rewrap.
