# 450 — The quantum is inclusive, and the hairline was short at exactly one pixel

**Finding.** `tiny-skia` takes its hairline for every stroke width up to *and including* one
device pixel, and a hairline carries `cos θ` of the rule's area — so every `1 w` stroke at the
page's own scale was drawn **29.3% short at 45°**, against a §10.7.4 `shall` that names strokes
with non-zero width by name.

**Date.** 2026-08-12.
**ADR.** [0285](../adr/0285-the-quantum-is-inclusive-and-the-hairline-was-short.md).
**Touched.** `crates/render-cpu/src/lib.rs`, `crates/render-cpu/tests/zero_area_fill.rs`,
`crates/render-quorra/tests/sub_pixel_coverage.rs`, `crates/render-quorra/tests/corpus.rs`,
`doc/conformance/ledger.toml` (§10.7.4), `doc/todo/11-shapes-that-still-disappear.md`,
`doc/todo/README.md`, `doc/adr/0285-*`, this file.

## What moved

One comparison, `<` to `<=`, split three ways: the general substitution takes the boundary, the
exact one stays strictly under it (snapping a one-pixel rule would be §10.7.5's stroke adjustment
without `/SA`), and the cap comes back at the quantum because ADR 0268's overstatement factor is
exactly 1 there. The `0 w` stroke follows the rule as a documented choice — §10.7.4 permits the
hairline, §8.4.3.2 states one device pixel, and `device_width` resolves it in the shared crate so
that both backends draw one mark.

## Measured on three instruments

| | before | after |
|---|---|---|
| oracle (1794 pages) | 905 / 68 / 786 | **identical** |
| step 7's ink sweep (854 pages) | tail at −5.734, −2.956, −1.150, −1.000 | unchanged; 33 rows moved, 31 up |
| quorra cross-backend | 917 / 35 / 5 / 17 | 915 / 37 / 5 / 17 |

The two-page cost is churn at bounds this project chose: `render-cpu` moved *onto* quorra's
construction, so the two rasterisers' coverage quanta now show on more edges. Against the
references two of the four newcomers improved — `bug1844583.pdf` ssim 0.6152 → 0.8738.

## And a test that failed with both constructions correct

`zero_area_fill.rs` placed its line at `50.3`, which puts the stroke's band a tenth of a pixel off
its row at scale 2 — below `tiny-skia`'s quarter-row sample quantum, so the snapped fill and the
unsnapped band came back byte-identical. `50.125` splits the ink at both scales. A test must be
placed off the rasteriser's sample grid, not merely off the pixel boundary.
