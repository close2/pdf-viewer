# Round 2158 — the thinnest line is one pixel every way, and a too-fine dash is its coverage

A fix round in the `gwg` worktree for audit 2155 (§8.4–§8.5), all five findings.

## What changed

- **B01, B03** (ADR 3333): `Stroke::is_thinnest_line`, `as_drawn` (a mitred join of the thinnest line
  is a bevel; raster asks per placement) and `strokes_in_device_space` (under unequal scales the line
  is `tiny-skia`'s hairline, Vello's path restated in device pixels, raster's device expansion).
- **B02** (ADR 3334): `MAX_DASHES` = 500 000; past it every backend strokes solid at
  `pattern_coverage` as an opacity, and `pdf-model` reports `LimitReached { "MAX_DASHES" }`.
- **B04** (ADR 3335): `read_dash`, shared by `d` and `/D`, sets nothing for a non-number and
  reports `StreamSyntaxFault::DashNotNumbers`. **D01** (ADR 3336): cap and join codes clipped to 0..=2.

## Evidence

- The four audit tests pass; `render-raster/tests/thinnest_line_on_every_backend.rs` holds the three
  backends to the same three cases. Planted back (the predicates and the bound off), the no-mitre
  case failed on all three backends, the width case on the processor and Vello (raster's two-point
  line takes its line primitive, already device-space), and the dash case **panicked Vello**: four
  million dashes asked for a 384 MB buffer against its 50 MB.
- Manifest: the four 2155 entries removed, 2148-B04's kept; 2155's statuses `fixed (...)`. Ledger:
  8.4.1, 8.4.3.2, 8.4.3.5, 8.4.3.6, 8.4.5, 8.5.3.2 re-read, kept `implemented`, notes and tests added.

**Gates.** Over `-p pdf-render -p pdf-model -p render-cpu -p render-raster -p render-gpu -p
raster-gpu`: fmt exit 0; clippy `-D warnings --all-targets` exit 0 on the fifth run (too-long
functions, `option_option`, `arithmetic_side_effects` on `kurbo` products, `assert_is_empty`);
`pdf-sandbox --bins` exit 0; nextest `--lib --bins --tests` (`xvfb-run -a`, RX 580) 4497 run, 4496
passed, 1 failed — 2148-B04's, the manifest's one ("failed 1, expected 1"); `render-cpu` doctests
exit 0; rustdoc exit 0; `fuzz` `cargo check --bins` exit 0; `cargo test -p conformance` failed
only on this record (no gates paragraph, then a path outside the tree), and `evidence` and
`records` pass after.

## Left

- §8.5.3.2's dot of a zero-width line under unequal scales is an ellipse (ADR 3333). Past the dash
  bound, raster's non-solid paints and knockout members are drawn at whole coverage (ADR 3334).
- The golden is the train's: 49 tracked documents state `0 w` beside a stroke and `issue7878.pdf`
  `/LW 0` (a census of the tracked corpus); their mitred corners may move.
