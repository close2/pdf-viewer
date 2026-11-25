# Round 2238 — a glyph's pattern in the glyph's space, and the simulated press's grey

A fix round in the `gwg` worktree for audit 2237's behaviour findings, on main `600550c3`.

## What changed

- B01, D02: `content/text/type3.rs`'s `draw_type3_glyph` swaps `Interpreter::base` for the glyph
  description's initial CTM (§9.6.4) around its run, whichever dictionary Issue #128's search ends
  in; the `base` field's doc, the shading and tiling placement comments and `run.rs`'s companion
  comment say "the parent content stream's default space" (§8.7.2). ADR 3653.
- B02: `pdf-colour`'s `SpotSet::named` answers a `Tints::GreyAsBlack` space as `Named::Process`
  before any name is compared, and `without_a_plane` stops at it; the paint and the overprint
  question both read `named` (§10.2). `intent_grey`'s `NAMES` doc no longer claims no page can
  name `/DeviceGray`. ADR 3654.
- B03: `GreyReading::AssumedInksBlack` (new) and `ColourSpace::grey_as_assumed_inks_black` (ADR
  2712's `Separation` over `DeviceCMYK`, which on the assumed press is its inks);
  `content::colour::output_intent_reading` takes `simulated` and gives it where the intent gave
  `Device`; `device_family` reads it after the intent's branch. Screen view unchanged. ADR 3655.
- Audit tests pass; their entries leave the manifest; B01–B03 and D02 `fixed` in `findings.json`.

## Ledger

Rows 8.3.2.4, 8.7.2, 9.6.4, 10.2, 10.3.2, 10.4.2.3, 10.8.3 re-read; each stays `implemented`, its
note and tests naming the construction and the audit test. No erratum changes a reading.

## Pixels

`raster_golden` held 972, moved 2, both by B01: `ContentStreamCycleType3insideType3.pdf` and
`ContentStreamNoCycleType3insideType3.pdf` p1, whose inner Type 3 glyph fills Helvetica with a
tiling pattern from its own font's `/Resources`; the cell now scales with the glyph (§8.7.2), so the
letters show a fine magenta texture where page-anchored cells left them mostly red; looked at, and
regenerated through the walk. B01's fixture, RX 580 live: CPU and `render-raster` (126, 0, 129),
`render-gpu` (127, 0, 128) at the centre — half blue on all three.

**Gates.** fmt 0; clippy `-D warnings --all-targets` of `-p pdf-colour -p pdf-model -p render-cpu
-p render-raster -p render-gpu -p viewer-qt` 0; `pdf-sandbox` 0; nextest `--lib --bins --tests` of
those six (`xvfb-run -a`) 3792 passed, 6 failed: `viewer-qt`'s media tests, `pdf-media-worker` not
built; with it built, the 6 passed; `audit-diff.py` otherwise "expected 0". rustdoc of `pdf-colour`
and `pdf-model` 0; `drawn_ahead_in_strips` 0; `cd fuzz && cargo check --bins` 0 (a public enum
gained a variant); `cargo test -p conformance` 0 once this record stated its gates.
