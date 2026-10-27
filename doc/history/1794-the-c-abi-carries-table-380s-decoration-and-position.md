# 1794 — The C ABI carries Table 380's decoration and position

Session 1794, a host round in batch 119; 2026-10-26. ADR 2422; ADR 2423 unused; no Q.

**Contract.** Slot 7: ADR 2384's decoration, colour and position through `viewer-ffi`'s C ABI,
versioned as its other additions are, and mapped in the WinUI window to UI Automation's text
attributes as far as that window's tree builds them.

**Moved.** No status: §14.8.5.4.4 stays `partial` (its unread five are a layout engine's); its row
names `abi/structure_reads.rs` and the new test, and its note says the colour crosses the ABI as an
absence and where N4's attributes are named.

**Premises.** *Three fields and one enum cross* — **held, as two enums**: `DecorationKind` and
`PositionKind`, each exhaustive over `pdf_model::structure`'s type with a `from_code`, through one
new entry point, `quorra_structure_style`; a symbol moves no `QUORRA_ABI_VERSION` (ADR 0576).
*The WinUI window's peer builds a tree* — **did not hold**: `hosts/windows/Quorra.App/` sets an
automation name on each widget's control and builds no UI Automation tree; RFC 0010's N4 owns
the peers. Nothing is written there; ADR 2422 names the eight attributes N4's text ranges answer,
an unstated colour as UI Automation's not-supported value rather than black. Linux: nothing built,
`accesskit_atspi_common` 0.19.1 has no overline or vertical offset.

**Found.** Through the C program, the application note's tagged page has 3 decorated nodes of 19.
Nothing in the workspace depends on `viewer-ffi` (`cargo tree -i`): the four windows cannot see it.

**Files.** New: ADR 2422, this record. Edited (`-U0`): `crates/viewer-ffi/` — `include/quorra.h`,
`src/kinds.rs`, `src/answers.rs`, `src/lib.rs`, `src/abi/structure_reads.rs`, `src/session.rs`
(fixture and test), `c/open_a_page.c`, `tests/header_and_library_agree.rs`,
`tests/every_query_reaches_the_abi.rs`, `tests/unsafe_position.rs`;
`hosts/windows/Quorra.Interop/Native.Constants.cs`, `Native.OtherHalf.cs`;
`doc/conformance/ledger.toml` (one row), `doc/todo/30`, `doc/state-of-play.md`. I worked in
`scratchpad/r1794/`.

**Gates.** `viewer-ffi`, bare: `cargo fmt --check` exit 0; `clippy -D warnings --all-targets` exit
0; `cargo nextest run` exit 0, 76 passed; `cargo test -p viewer-ffi --no-fail-fast` exit 0, 78
passed; rustdoc `-D warnings --document-private-items` exit 0 (the workspace walk not run). The
header's checks are its tests, `header_and_library_agree` and `header_and_interop_agree`, both in
those runs; `tools/dotnet-check.sh` exit 0, 0 errors. `cargo test -p conformance --no-fail-fast`:
exit 101 with 68 targets green and `round_numbers` red on a sibling's new example, then exit 0
with 69 green on a re-run. No pixel and no figure moves, so the arms, `raster_golden` and the clock gates are
not owed.
