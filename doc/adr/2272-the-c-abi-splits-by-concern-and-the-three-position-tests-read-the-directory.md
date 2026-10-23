# 2272 — The C ABI splits by concern, and the three position tests read the directory

Session 1719, the standing code-quality slot of batch 110. Status: implemented; no symbol, signature or count moved.

`crates/viewer-ffi/src/abi.rs` was the largest hand-written source file (9 908 lines, 355 entry
points). It is now the module root (88 lines: the pointer contract, the lint lift, the module list
and the re-exports) over eighteen children in `src/abi/`:
`layouts` (the structs passed by value and the `QUORRA_RICH_*` constants), `identity`, `commands`,
`event_stream` (events and their field accessors), `rendering` (request, raster, oversized window,
preparation), `queries` (the synchronous queries), `conversion` (ISO 19005), `panel_reads`,
`pointer`, `forms`, `edits` (the four edits and the bytes out), `host` (clock, policy, printing,
the toolkit's controls and the keyboard), `page_reads`, `popup_reads`, `structure_reads`,
`collection_reads`, `window_calls` (`quorra_window`) and `shared`.

**The rules the split keeps.** `abi.rs` holds the one `#![allow(unsafe_op_in_unsafe_fn)]`; an inner
attribute covers its descendants, so the lift is still stated once and `lib.rs`'s single `expect`
still names `pub mod abi;`. Every child is re-exported with `pub use self::child::*`, so
`viewer_ffi::abi::quorra_…`, `Pdfv…` and `QUORRA_…` are the paths they were. The six helpers more
than one child calls (`owned_text`, `copy_out`, `copy_floats`, `hand_over`, `script_answer`,
`write_script_question`) are `pub(super)` in `shared`, which is not re-exported; the three that
only the popup reads call went to `popup_reads`. Each child imports exactly the names its code
uses.

**The tests moved with the files.** `tests/unsafe_position.rs` named the file `abi.rs`; it now
keys on the path under `src/` and accepts `abi.rs` and `abi/…`, and it counts a
`pub(super) unsafe fn` as the helper signature it is (355 attributes and 336 signatures, as before).
`header_and_library_agree.rs` and `every_query_reaches_the_abi.rs` read the root and the children
joined. A new `unsafe` anywhere outside `src/abi*` still fails the first; an entry point in a child
still fails the second without a header line.

**What did not move.** No entry point's body, signature or order within its child; the header, the
C driver and the counts. The only changed lines are `use` lines, the module headers, the
`pub(super)` prefix on the six helpers, and 71 intra-doc links, which resolved in a single module and
now name the sibling by path (`[`name`](super::name)`).

**Proof.** The multiset of lines of `abi.rs` at HEAD against the union of the new files, whitespace
lines excluded, differs only by those four classes of line. `cargo test -p viewer-ffi` passes every
integration test (the C driver among them); `cargo doc -p viewer-ffi --no-deps` has no unresolved
link; clippy with warnings denied and `cargo fmt --check` are clean.
