# Round 2267 — §10.8.3's simulated press takes all four transfer functions

A fix round in the `lane3` worktree, taking review 2248's fix group 2: B03 (the press took the
screen's three functions on the composed sRGB pixel), B04 (a `DeviceGray` colour on the press went
through all of them) and D01 (comments that named the screen as the only device).

## The fix

- **B03, ADR 3769.** `pdf_colour::Transfer` keeps the screen's three, the press's four process
  functions in Table 57's order and a spot colourant's (a single function or a halftone's; an array
  of four states none). A Type 5 halftone is read by each device's §10.6.5.6 names.
  `pdf_render::TransferMap` carries every table, and the channel carries a `TransferDevice`, which
  `draw_separated` sets to `SimulatedPress`. `resolve_press_transfers` maps the chromatic, black and
  spot planes before `blending::resolve`/`separation::resolve`, choosing per pixel by the same walk
  as `resolve_transfers`. Lanes: `render-cpu` and `render_raster::finish` (its `rasterize` and the
  window's present); `render-gpu` refuses a channel on either device.
- **B04, ADR 3770.** A fill or stroke in the space `grey_reading`/`intent_grey` give `DeviceGray`
  on a CMYK device carries `Transfer::grey_alone`. `output_intent.rs` is unchanged.
- **D01.** `transfer.rs`, `draw_mark`'s overprint paragraph and `render-gpu`'s comment name their
  device. ADR 3771 records that an overprinting mark on the press still takes the default.

## Ledger

Rows before and after: 10.5, 10.8.3 and 11.7.5.2 `implemented`, unchanged. 10.5's note no longer
calls its DeviceGray sentence inapplicable; it gains the new code and tests.

**Gates.** Rebased on a59e8c437, over `-p pdf-colour -p pdf-model -p pdf-render -p render-cpu
-p render-raster -p render-gpu -p viewer-core -p test-scenes`: fmt 0, clippy `-D warnings
--all-targets` 0, rustdoc 0, `pdf-sandbox` 0, nextest `--lib --bins --tests` 4796 passed, 0
failed (the 2248 manifest is empty), fuzz 0. `press_transfers` draws three lanes on the RX 580.
`raster_golden` moved one entry, `issue6931_reduced.pdf` p1, list only (the map carries the press's
tables), re-held; ahead-in-strips 0. Launch 0 (within every band).
Turn 0: 33 figures banded, 33 judged, 0 outside, each page within a few percent of main's
d94ef46b3 log; a first run under other rounds' load put 5 outside, none on a page with a
transfer channel. Conformance 0.

**Habit.** The conformance gate attributes a blockquote to the nearest clause before it in the
comment, so a quote of §10.5 in a comment that names §10.8.3 first needs its own citation line.
