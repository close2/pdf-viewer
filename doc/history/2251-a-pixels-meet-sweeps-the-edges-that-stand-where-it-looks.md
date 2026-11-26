# Round 2251 — a pixel's exact meet sweeps the edges that stand where it looks

A performance round in `lane3` on main `153f4e7d6`: `ContentStreamNoCycleType3insideType3.pdf` p1
turned in 4.9 s against its 20–33 ms band since ADR 3653.
## Where the time went

The page states 12 224 commands where its band was taken on 152: 3 054 tiling cells of about
2 × 1.15 pt, each a Helvetica "ba" filled and stroked. A stroke is a glyph two device pixels high
expanded into about 1 200 overlapping pieces. Measured on one page, three zooms, three draws each:
- `render-raster`: 5.8–7.3 s at zoom 1; `perf` put 48 % in `encode::meet::exact_areas`, which
  compared every pair of the thousand edges through a cut pixel and scanned every edge per band.
- CPU oracle: 5.3–10.6 s at zoom 1, 64 % in `area::union`'s pair search and settling — the same
  shape, the union of the pieces built exactly (ADR 2122).
- `render-gpu` (vello): 127–202 ms; it estimates coverage and measures no union.
The turn log's 300 MHz sclk is the device idle while the encoder works on the processor.

## What changed

- `raster-gpu`'s `area_in_pixel` sweeps the edges through a pixel by their tops: `crossings`
  compares an edge only with the edges still standing at its top, and each band reads only the
  edges spanning its middle, kept in `through`'s order. Every pair, cut and boundary order is what
  the full scan made, so every area is the same bits; `the_sweeps_measure_the_same_bits_as_every_pair`
  holds the old construction beside it. ADR 3705.
- Q1073 asks whether this page's band moves or its pixels may.

## Figures and pixels

Raster at zoom 1: 2.5–2.8 s from 5.8–7.3 s; its raw frames hash identical before and after at
zooms 1, 1.5 and 3; the CPU oracle and `render-gpu` are untouched. The page's turn child: turn
2282 ms (4910), seventh 2380 (4913), step 876 (1096), still outside (full turn test, 33 judged).
`personwithdog`, `issue13931` and EC3 p101 make no exact meet (counted: zero pixels), so their
moves are the processor's load; `issue13931`'s is its `interp`. Launch: 0 outside.

**Ledger.** 10.7.4 (`departed`), 8.7.3.1 (`implemented`) re-read, unchanged; no erratum.

**Gates.** fmt 0; clippy `-D warnings --all-targets` of `-p raster-gpu -p pdf-model -p render-cpu
-p render-raster -p render-gpu -p viewer-qt` 0; `pdf-sandbox` 0; nextest `--lib --bins --tests` of
those six (`xvfb-run -a`) 4384 passed, 5 failed, the five `10.5–10.8` audit tests the manifest
expects (`audit-diff.py`: no difference); rustdoc of `raster-gpu` 0; `cargo test -p conformance` 0.
