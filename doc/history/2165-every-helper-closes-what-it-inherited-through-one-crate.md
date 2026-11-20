# Round 2165 — every helper closes what it inherited through one crate

The owner's A979 and A974 (2026-11-20): the tree's unsafe sweep red on main since
`pdf-media-worker` and `quorra-sound` copied ADR 1945's close, and audit 2148's B04 test sending the
name where ADR 3311 lets an edit name the widget (ADRs 3361, 3362).

## What it did

- `inherited-descriptors`, a new crate of one function: `keep_only_the_standard_descriptors`, the
  `close_range(3, ~0, 0)` and its one lift. The five programs — `pdf-sandbox-worker`,
  `pdf-view-worker`, `pdf-script-worker`, `pdf-media-worker`, `quorra-sound` — call it as `main`'s
  first statement and now `forbid(unsafe_code)`; their copies and four crates' `libc` are gone.
  `viewer-qt`'s sweep names five crates where it named seven, the shared crate covering both of
  A979's helpers. `tests/callers.rs` reads the tree back: the crate's one lift and one `unsafe`,
  and exactly the five sources naming it, each calling it once and first. `pdf-sandbox`'s
  `inherited_descriptors.rs` keeps the behavioural test alone.
- B04's test sends `Entered::Selected(second.annotation)`, assertions unchanged, its doc comment
  rewritten to what a host sends; it passes. B04 leaves `expected-failures-current.json`; 2148's
  `findings.json` says fixed (status only).
- `Clicked::Toggles` carries `entered` (`Selected(widget)` on, `Text("Off")` off), `toggling` takes
  the widget, and `quorra`'s pointer, GTK's and Qt's controls and accessibility clicks and the C
  ABI's `toggle_field` send it. `host_mappings` gains the two-same-named-radios click test.

**Left, in order.** Whether `quorra-transform-worker` and `pdf-vfs-worker`, which close nothing,
are started beside a leaking library (ADR 1907's question, then one call each); the manifest's five
2161 entries are round 2163's.

## Ledger and gates

**Ledger.** No status moves: §12.7.5.2.4 stays `implemented` (ADR 3311); §12.7.5.2.3's row names
the renamed `viewer-host` unit test, `implemented` -> `implemented`.
**Gates.** One `-p` set — `inherited-descriptors pdf-sandbox viewer-confined pdf-script-worker
pdf-media-worker quorra-sound viewer-qt pdf-model viewer-host viewer-gtk viewer-ffi viewer-ui`: fmt
exit 0; clippy `-D warnings --all-targets` exit 0; `pdf-sandbox --bins` exit 0; nextest `--lib
--bins --tests` under `xvfb-run -a`: 3918 run, 3913 passed, 28 skipped, 5 failed — the manifest's
five 2161 entries and nothing else (`audit-diff.py`: failed 5, expected 5). Windows (gnu, msvc):
clippy of `inherited-descriptors` 0, `cargo check -p pdf-sandbox --all-targets` 0; clippy of
`pdf-sandbox` 101 on `lockdown.rs`'s `apply_for_decoder_within` (`unnecessary_wraps`), unchanged
since df503353 and left, no lint-only edit. rustdoc `-D warnings --document-private-items` of eight crates 0. `cargo test -p conformance
--no-fail-fast` 101 on `records` alone (this record's draft), then `--test records` 0.
