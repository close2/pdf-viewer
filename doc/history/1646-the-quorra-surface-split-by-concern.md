# 1646 — `App`'s surface split by concern

Session 1646, slot 8 of batch one hundred, a docs refactoring round (Sonnet 5.5); 2026-10-20. ADR 2128
(2129 unused). The owner's words: "Refactor god classes,..." and "Do not fix any bugs." None was fixed.
`doc/todo/74` entry 9, `impl App` in `crates/viewer-ui/src/bin/quorra/surface.rs`.

**Built.** `surface.rs` (1 783 to 305 lines) keeps `Surface`, `State`, the four accessors,
`redraw_requested` and the free functions `predicted`, `in_the_window`, `wake_on`; five modules under
`surface/` hold `impl App` by concern: `device_frame` (518 lines), `device_adoption` (290),
`processor_frame` (396), `bring_up` (248) and `settling` (70).

**The premise that did not hold.** None; the entry said to read the block against `window.rs` and
`viewer-device`'s `renderer/` first, and the cut follows the two frame paths and left both untouched.

**The proof that nothing moved.** Line multiset of the old file against the root and five modules:
removed-only 11 lines (ten signatures that gained `pub(super)`, one import list), added-only 75 (module
docs, `use`/`mod` lines, five `impl` wrappers, the same ten signatures, one signature `rustfmt` broke
across lines). Ten methods are `pub(super)`, each widened from E0624; `on_the_device` was widened and
narrowed back (habit 137). `viewer-ui` is a leaf crate (`cargo tree -i viewer-ui`: nothing above it),
so the six arms are not owed.

**Ledger.** No row, teaching diagram or `doc/crate-map.md` row names the file; none re-pointed.
`doc/todo/74` entry 9 is built.

**Found, not fixed.** No bug. `cargo test -p conformance` fails `round_numbers` on a sibling's scratch
file, `crates/pdf-model/examples/r1640_isolate.rs:1` ("Scratch instrument of round 1640, removed before
the round reports"), not mine.

**Unfinished.** Entries 10 and 11.

**Gates.** `cargo fmt --check -p viewer-ui`: 0. `RUSTFLAGS=-D warnings cargo clippy -p viewer-ui
--all-targets`: 0. `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --document-private-items -p viewer-ui
--bins`: 0. `cargo test -p viewer-ui --bin quorra`: 0, 26 passed. `cargo test -p conformance
--no-fail-fast`: 101, one failure (`round_numbers`, the sibling file above), every other binary
passed. Duration: about 1 500 s.
