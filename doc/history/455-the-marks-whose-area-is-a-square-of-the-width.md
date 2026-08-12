# 455 — The marks whose area is a square of the width

**Finding.** §10.7.4's substitute for a sub-pixel mark restores a *body* exactly and a **cap** only
once, because a cap's area goes as the square of the line width — so ADR 0268 dropped the cap, and
the same arithmetic that justified dropping it says how to keep it. Measuring that found a second
mark of the same family nobody had looked at: §8.5.3.2's dot, which vanished outright under a fifth
of a device pixel and was reported by nothing.

**Date.** 2026-08-12.
**ADR.** [0290](../adr/0290-the-marks-whose-area-is-a-square-of-the-width.md).
**Touched.** `crates/pdf-render/src/sub_pixel.rs` (`enlarged_mark`, `sub_pixel_caps`),
`crates/pdf-render/src/degenerate.rs` (`KAPPA` shared), `crates/pdf-render/src/lib.rs`,
`crates/render-cpu/src/lib.rs`, `crates/render-quorra/examples/sub_pixel_marks.rs` (sections 5
and 6), `crates/render-quorra/tests/sub_pixel_coverage.rs` (two gates),
`crates/pdf-model/examples/sub_pixel_width_census.rs` (new), `crates/pdf-model/tests/oracle.rs`,
`doc/conformance/ledger.toml` (§8.4.3.3, §8.5.3.2, §10.7.4), `doc/QUORRA_FEEDBACK.md` (§21),
`doc/todo/11-shapes-that-still-disappear.md`, `doc/todo/README.md`, `doc/adr/0290-*`, this file.

## Measured

Ink against the mark's own area, on the processor, at scale 1 — `sub_pixel_marks` sections 5 and 6:

```text
  cap       angle   length   width      before     after    its own area
  Round         0     0.15    0.50      0.0000    0.2353         0.2713
  Round         0     0.50    0.50      0.2510    0.4392         0.4463
  Round        30     0.50    0.50      0.1882    0.3765         0.4463
  Square        0     0.15    0.50      0.0000    0.2510         0.3250
  Square        0     0.50    0.50      0.2510    0.4863         0.5000
  §8.5.3.2's dot, diameter 0.10        0.0000    0.0157         0.0079
  §8.5.3.2's dot, diameter 0.20        0.0000    0.0314         0.0314
  §8.5.3.2's dot, diameter 0.50        0.2510    0.1882         0.1963
```

Every oracle verdict count identical over 1794 pages; 36 per-page lines moved and 35 of them in the
third decimal place. `issue12295.pdf` page 1 moved towards the references — worst mean 5.61 → 5.55,
ssim 0.7486 → 0.7494. `doc/todo/00`'s step-7 sweep, run before and after over all 786 ambiguous
pages: **49 rows moved and every one of them up**, the negative tail unchanged at twenty names.
Cross-backend gate unmoved at 915/37/5/17. Corpus gate and the whole workspace suite pass.

Instructions, `callgrind_rasterise`, 20 rasterisations: **+0.19%** on page 101 of ISO 32000-2 and
**+146.8%** on `issue12295.pdf` page 1, whose 65 859 sub-pixel round-capped strokes are each a
second fill. Priced in `doc/todo/11`, with the one-draw construction that would take it back.

## Found while measuring, and not this tree's

The ladder reads the device too, and two rows of it are quorra's: **it draws no round cap at any
width** — a 5-unit-wide rule reads the butt-capped answer to the last digit — and **it flattens a
small circle into a polygon inscribed in it**, 0.5020 against `π/4` at one device pixel, which is
the inscribed square exactly. Written up as section 21 of `doc/QUORRA_FEEDBACK.md` with the
reproduction, and the two gate rows they cover hold the processor only, with the reason in the
test's own comment: a gate on a row the device cannot draw would ratchet a defect rather than a
requirement.

## What is left, and it is the raster's

On `issue12295.pdf` the caps that now draw are 1.14 levels of 255 of geometry and 0.133 of a level
landed, because each cap's ink is about half a level on each of a few pixels and half a level is
what an eight-bit raster rounds away. The mark no longer disappears, which is what the clause
requires; what lands is what the raster can hold. `pdf-model/examples/sub_pixel_width_census` is
the instrument that says so — it prints a page's own sub-pixel widths and what their caps are worth
— and it exists because the arithmetic in this round's first draft was wrong twice before it was
measured.
