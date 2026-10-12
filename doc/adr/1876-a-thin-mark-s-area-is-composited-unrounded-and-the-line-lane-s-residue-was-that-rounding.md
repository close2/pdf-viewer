# 1876 — A thin mark's area is composited unrounded, and the line lane's residue was that rounding

Session 1519. Status: **accepted** and **built**. Amends ADR 1829 §3's first sentence; for the
project owner's performance target of 2026-10-10 (round 1499's residue, `doc/todo/69` §2). Context:
ADRs 0418, 1374, 1829, 1835, 1857; raster ADR 0082; traps 1, 13, 62. Code:
`crates/render-cpu/src/thin_segment.rs` (`paint_segment`, `source_over`). Test:
`render-cpu/tests/thin_segment_ladder.rs::a_mark_is_composited_at_its_area_and_rounded_once`.
Clauses: §11.3.6, §10.7.4, §8.4.3.2. No ledger row moves.

## 1. The hypothesis, which did not hold

Round 1499 measured the device's line lane against ADR 1829's oracle on the witness at up to 15
levels, mean 0.19, with single marks agreeing to under a level, and named overlap order, each side
compositing the dots into eight-bit pixels in its own order. **It is not order.** Two overlapping
dots of four widths at two placements, two colours, drawn in both orders: each backend's two orders
differ exactly as §11.3.6 says they must, and the device and the processor agree in either order
to one level (round 1519's scratch `residue order`).

## 2. What the residue is, measured in isolation (trap 62)

The witness at the fit (1417 × 1001) holds 4 396 311 marks `thin_segment` admits and 12 094 others.
The round's instrument drew the admitted marks alone on both backends. It also composed the same
areas in `f64`, one mark at a time, from §11.4.7's transparent page group with the medium put
under it last. With switches: the area rounded to a level first, and the stored pixel rounded per
composite. One model is the area rounded to a level and then the store rounded. It reproduces the
processor to 20 bytes of one level. One rounding, the store's alone, reproduces the device to mean
0.021, worst 5, the hardware's unorm blend against `f32`. So the whole difference between the lanes
was ADR 1829 §3's level taken off each area before compositing. The processor did it, the device
did not. Against the exact composition the processor was the worse of the two. Mean |d| 0.359,
mean signed −0.259 (darker), worst 14.05. The device: 0.277, +0.029, 11.89. A level taken per mark
is an error the marks of one column share, because they sit at one horizontal phase, so it adds
over the run instead of cancelling.

## 3. The decision

**The processor composites a thin mark's area as measured and rounds once, when the pixel is
stored.** The clip and soft mask meet it as factors, each byte read as a level over 255. The floor
of ADR 1374 stays: a mark under a level everywhere is still stated at one level in each pixel it
touches. A coverage that is a whole level stores the byte `tiny-skia`'s pipeline stores. Nothing
in §11.3.6 asks for the first rounding. It came from matching the library's eight-bit mask, which
this kernel never hands a mark to. The store's rounding is both lanes' eight-bit raster, and stays
the lanes' shared limit.

## 4. Evidence

- **The witness at the fit**, HEAD export against HEAD plus this: device (`Cpu` coverage) against
  oracle, whole page, 619 361 → 265 222 bytes differ, mean 0.1893 → 0.0715 levels, worst 14 → 13.
  The worst is now the 12 094 other marks' (circles and glyphs, 13 levels there before and after).
  The other two lanes the same way: `Compute` 555 598 → 201 117 bytes, mean 0.1584 → 0.0405,
  worst 14 → 13, SSIM 0.99970 → 0.99994; `Gpu` 767 343 → 417 157, mean 0.5061 → 0.3895, worst 39
  → 39 (its own coverage's, elsewhere), SSIM 0.99884 → 0.99907.
  The thin marks alone: 699 162 → 109 386 bytes, worst 19 → 5, mean 0.287 → 0.021. Oracle against
  the exact composition: mean |d| 0.359 → 0.280, signed −0.259 → +0.014, worst 14.05 → 11.89.
  The processor's own page: 264 945 pixels move, at most 14 levels, ink −0.143 %, the dark bias
  leaving the racks' shading columns; looked at beside HEAD's, the same drawing.
- **The ladder**: grey marks over white at 243 placements store `round(255 · (1 − ½ · area))` to
  the byte. The control counts the pixels where a level taken first stores another byte, and more
  than 50 must. Planted back, the rung fails at (3, 9), 248 for 247. Reverted.
- **`raster_golden`**: held 970, moved 4, all "raster only": `bug1978317.pdf` 8 812 pixels by one
  level, ink +0.046 %; `issue12295.pdf` 3 043 pixels by up to 5, ink −0.068 %; `issue12810.pdf`
  15 644 by up to 11 (with ADR 1875's dots); `issue14953.pdf` one pixel by one. Device against
  oracle improves on all four. Looked at, beside HEAD's. The division test is unchanged at 176
  pages, ≤ 1 308 pixels, ≤ 3 levels.

## 5. What is left

The store's own rounding: a mark under half a level in a pixel from an empty backdrop leaves no
trace in either lane, so a run of such slivers loses up to 11.89 levels against the exact
composition on the witness. Closing it needs a wider accumulation than eight bits on both sides,
and raster ADR 0082's bound holds it meanwhile. The floor (ADR 1374) has no counterpart on the
device. No witness mark reaches it, and it is a difference only for marks under a level everywhere.
