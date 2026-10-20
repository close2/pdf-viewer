# 1626 — A window crosses to C as the loop the library runs

Session 1626, slot 4 of batch ninety-eight, a "host" round; 2026-10-20. RFC 0010 section 6's N2,
its first round; ADR 2088 (2089 unused). No Q file: no owner's word was needed. No ledger row moved.

**Built.** `crates/viewer-ffi/src/window.rs` (with `window/owed.rs`, `window/asked.rs`):
`NativeWindow` implemented for a C caller — the document opened by path on a thread and joined at
the first resize, pages on the drawing thread, a tick at the interval the window states, a key
through `viewer_host::decide` with the trait's acts carried out and the widget acts handed back, the
pointer, Annex O's search a page per tick, the strip of open documents, §7.6.4.1's prompt and the
*ask* level's one question. Ten `QUORRA_OWED_*` kinds and their count, `QUORRA_UNREADABLE` (9), 32
entry points: 268 to 300. The C# for all of them, `Native.Window.cs` new. RFC 0010 gains section 3.2a.
`c/drive_a_window.c` is a toolkit's loop, one call per thing that happened to the window.

**The premise and the hypothesis.** The premise held: `NativeWindow`'s required methods are what a
toolkit answers its own way. The RFC's shape did not: a table of callbacks is what ADR 0247 refused
for this ABI, and `say` is called inside `drain` while the window is borrowed, so the methods owe
records instead (RFC section 3.2a) — `viewer-qt`'s flags, as a batch. The hypothesis held for the
module and the loop (one call per event), not for the count: 32 entry points, not a dozen.

**Calibrated.** `float scale` planted as `double` in `quorra_window_resize`'s C# and
`QUORRA_OWED_PLACE` as 8: `header_and_interop_agree` named both; reversed. The search's page was
checked against `pdftotext` as evidence: no "Black Point" on page 2, three on page 3.

**Unfinished, and N2's second round.** The panels' rows, the placed controls and their commits, the
find bar's matches and highlights, the selection's quadrilaterals; the archive chooser, a script's
`app.alert` and a print grant are each declined aloud; the window opens at `Settings`' defaults
until section 5's policy parser crosses. The carrying-out of a document's requests is a third copy
beside the two toolkits' (ADR 2088 section 4). No `dotnet` here: the C# is compiled by CI only.

**Gates.** `rustfmt --check --edition 2024` on every touched Rust file and `cargo fmt --check -p
viewer-ffi`: 0. `RUSTFLAGS="-D warnings" cargo clippy -p viewer-ffi --all-targets`: 0. `cargo
nextest run -p viewer-ffi`: 0, 53 of 53, both C programs (`-Werror`) and `header_and_interop_agree`
(4 of 4) among them. `cargo test -p conformance --no-fail-fast`: 0. `tools/state.sh --round 1626
hosts`: 0, Command 45 of 45, Query 35 of 36 (`LogicalSelection`, a sibling's session edit), 300 entry
points. `tools/state.sh --round 1626 docs`: 0. Rustdoc `-D warnings` on `viewer-ffi`: 43 located, as
before, none in a line written here. `cargo tree -i viewer-ffi --workspace` names no dependant, so no
window's graph moved (habit 119). Duration: about 3388 s.
