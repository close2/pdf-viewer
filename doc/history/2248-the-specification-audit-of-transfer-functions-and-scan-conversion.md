# Round 2248 — the nineteenth specification audit: §10.5 to §10.8

An audit round in the `audit` worktree, in the shape of review 2038 and audits 2128 to 2243: every
`shall` and `should` of §10.5 to §10.8 (transfer functions, halftones as far as a screen still owes
them, scan conversion and stroke adjustment, rendering for separations) against `pdf-model`'s
interpreter, the CPU oracle, `render-raster` (both its `rasterize` and the window's present) and
`render-gpu`. The `viewer-device` route to the present was read by a Sonnet helper and its one call
checked here. It fixes nothing.

## What it found

- `doc/reviews/2248-specification-audit-10-5-10-8.md` and its directory: 5 B, 0 T, 1 D.
- B01 the window's present draws a page without its transfer functions (`rasterize` applies them).
  B02 it draws §10.8.3's simulated press without its black list and spot planes. B03 the simulated
  CMYK press applies the transfer as an RGB device's, never reading the fourth function. B04 a
  `DeviceGray` colour there takes the RGB functions, not the gray one alone. B05 `/SA true` fits
  only strokes under 1.5 device pixels. D01 three comments name the screen as the only device.
- Five failing tests with controls: `pdf-model/tests/rendering_10_5_10_8_spec_audit.rs`
  (`content_suite`) and `render-raster/tests/window_frame_10_5_10_8_spec_audit.rs` (the RX 580);
  listed in the audit's `expected-failures.json` and appended to `expected-failures-current.json`.

## Ledger

Rows before and after: unchanged (§10.5, §10.6.5.x, §10.7.x but 10.7.4, §10.8.x `implemented`;
§10.6.1 to §10.6.4 `inapplicable`; 10.7.4 `departed`). The findings contradict 10.5, 10.6.1,
10.7.5, 10.8.3 and 11.7.5.2.

**Gates.** Over `-p pdf-model -p render-cpu -p render-raster -p render-gpu -p viewer-qt`: fmt 0;
clippy `-D warnings --all-targets` 101 once on an unfulfilled `doc_markdown` expectation in the new
GPU test, fixed, then 0. `pdf-sandbox` 0. nextest `--lib --bins --tests` (`xvfb-run -a`, GPU live):
3602 run, 5 failed — exactly the manifest's (`audit-diff.py`: "failed 5, expected 5"). rustdoc of
`pdf-model` and `render-raster` 0. `cargo test -p conformance --no-fail-fast` 101 on
this record's gates sentence alone (`records`), fixed, then `records` 0. Fuzz not owed.

## Habit

Every route to the device is a route. The transfer channel was proved equal across `render-cpu` and
`render-raster` by tests that both call `rasterize`; the window never calls it. A clause applied
"over the finished raster" is owed by every path that finishes one, and a test of the lane is not a
test of the window.
