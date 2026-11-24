# 2219 — The device's share a median, and a clip's picture through the C ABI

Round 2219. ADRs 3577, 3578.

## What changed
- The memory figure (ADR 3577): no commit raised `xfa_filled_imm1344e`'s `peak_anon_mib`. Over 45
  gate logs, rounds 1954 to 2218, the first page's own term read 15.0 to 15.6 MiB and the device's
  one bring-up child 11.8 to 13.7; 2215's 3.469 was 15.3 less 11.8. Ten direct pairs: first page
  15.25 to 15.54, bring-up 12.32 to 13.17. The device's share is now the median of nine bring-up
  children with huge pages off, the first page's own condition; no band moves.
- The C ABI's frame sink (ADR 3578): `quorra_settings_show_pictures` opens a window with
  `Hearing::of_a_showing_window`; `quorra_window_picture` and `_copy` hand the `/F` part as RGBA,
  placed from `Query::Screen`; `QUORRA_REPAINT_PICTURE` (8) says it changed; the host's timer at
  `quorra_window_interval` takes `next_look_unwoken`. Escape, Space, Table 218 and close reach it.
  Header, C# declarations and constants; the WinUI host states the word and draws the picture in
  one `Image` over the pages (C# not compiled here). `Placed::bounding_box` and `is_whole` are
  shared with Qt.

## Tests
`viewer-ffi` `src/window/picture_tests.rs` (end, pause, resume, Space, Escape, stop, close; a
host drawing none is handed the clip); `c/drive_a_window.c`; header and unsafe counts 390, 371.

## Ledger
No status moves. §12.6.4.14 and §13.2.1: notes, code and tests restated for the C ABI.

**Gates.** Over `viewer-ffi`, `viewer-host`, `viewer-core`, `viewer-ui`, `viewer-qt`, `pdf-syntax`,
`pdf-model`: fmt 0; clippy `-D warnings --all-targets` 0; `cargo build -p pdf-sandbox -p
pdf-media-worker --bins` 0; nextest `--lib --bins --tests` under `xvfb-run -a` 4831 of 4833, the two
the header test's counts, fixed, then `viewer-ffi` 96 of 96; none against the manifest; rustdoc of
the five viewer crates 0; fuzz `cargo check --bins` 0; `cargo test -p conformance` 0 after the
record's gates.
`state.sh launch` before 0 (`xfa_filled` 2.1 beyond one child's 13.0), after 0, 36 banded, none
outside (`xfa_filled` 2.3 beyond the median 13.03 of nine, 12.47 to 13.73).

## Left, in order
1. A device allocation band of its own (ADR 2667's open point), with the driver's figures.
2. The WinUI picture path compiled and driven by CI's Windows job.

**Trap.** A difference of two measured terms is judged only as tightly as its noisier term: read
each term over the record before looking for the commit that moved the difference.
