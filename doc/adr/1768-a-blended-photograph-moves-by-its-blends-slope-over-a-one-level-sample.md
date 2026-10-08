# 1768 — A blended photograph moves by its blend's slope over a one-level sample, and the sample is held to the bound

Session 1467. Status: **accepted** and **built**. Tests the reading ADR 1758 section 2 left unproved
and section 5 asked a ladder of offsets for. It does not change ADR 1758's decision; it says what
the first of its eight ceilings rests on. Context: ADRs 0047, 0219, 1742, 1758; traps 13, 62, 66,
134. Code: `crates/pdf-model/tests/strip_parallelism.rs`
(`a_blended_photograph_moves_only_where_one_of_its_samples_moved`), `crates/pdf-model/tests/raster_golden.rs`
(the first note of `PAST_THE_BOUND`). Instrument: `scratchpad/r1467/probe/src/main.rs` (sections A
to E), its output in `scratchpad/r1467/explain.txt`.

## 1. The page, and the premise that did not hold

`blendmode.pdf` page 1 draws sixteen cells, each a 100 × 100 RGB photograph under Normal and a
second one over it under one of the sixteen modes, both at an 80 × 80 placement. Both images' soft
masks decode to a constant 255, so inside a cell every pixel is `B(Cb, Cs)` of two opaque samples.

The brief's premise was that the **Saturation** cell reaches three levels. It does not: over the
pixels each cell's images cover whole, Saturation moves 14 to 18 pixels by **2** levels at every one
of the golden's four divisions. The three-level pixels are one in the **Hue** cell, (234, 436), and
one in the **ColorDodge** cell, (147, 575), each at two, four and eight strips.

## 2. Whose sample moved

Each cell's backdrop was drawn alone and its source alone, under Normal, by making the other image
transparent (`alpha` 0) on the full list. The list is kept whole on purpose: a list holding only
the cell's two images is one the strip planner refuses to divide (its replay bound), so a first
decomposition that way measured a page drawn in one strip and found nothing. That was an instrument
with nothing to see (trap 13), and the full list is what made it see.

Under strips, every input that moved moved by **one level in one channel**, on 0 to 26 pixels a
cell. In the four cells `render-cpu` composites itself (Table 135's modes, ADR 0047), every composed
pixel that moved sits where its backdrop or its source moved, at every division. The worst three:

- **ColorDodge (147, 575)**: backdrop green 76 → 77 under a source of 177, composed 248 → 251.
  Table 134's ColorDodge divides the backdrop by `1 − Cs`: 76 × 255 / 78 = 248.5 and 77 × 255 / 78 =
  251.7, a slope of 3.27.
- **Hue (234, 436)**: source (214, 216, 223), green 216 → 217, composed (−3, +2, −3). `SetSat` of
  §11.3.5.3 divides by the source's `Cmax − Cmin`, nine levels here.
- **Saturation (281, 559)**: backdrop red 66 → 67, composed red and blue +2.

In the twelve separable cells, which `tiny-skia` composites, one to three composed pixels a run move
with neither input moving: the library blends the source before rounding it, so a move under a
level in the source reaches the blend and not the source drawn alone.

## 3. The ladder

The whole page at one strip with `ty` moved by `n` units in the last place (`2⁻¹⁴` px at
`f` = 841.89), each cell against `n` = 0. *in* is the inputs' moved pixels and worst level, *out*
the composed cell's, *gain* the most any composed pixel moved per level of its input:

| n | px | Normal in / out | Saturation in / out, gain | Hue gain | ColorDodge gain |
|---|---|---|---|---|---|
| 1 | 0.00006 | 43/1 · 16/1 | 64/1 · 48/4, 4 | 4 | 6 |
| 4 | 0.00024 | 137/1 · 42/1 | 149/1 · 116/6, 6 | 4 | 6 |
| 16 | 0.00098 | 509/1 · 144/1 | 509/1 · 359/48, 48 | 8 | 6 |
| 64 | 0.0039 | 1713/1 · 542/1 | 1656/1 · 1207/69, 69 | 26 | 8 |
| 256 | 0.0156 | 4242/3 · 1850/2 | 4216/4 · 3426/93, 93 | 67 | 12 |
| 1024 | 0.0625 | 5975/14 · 3981/8 | 5972/13 · 5526/96, 69 | 50 | 12.5 |
| 8192 | 0.5 | 6241/102 · 6026/68 | 6240/105 · 6223/145, 37 | 34 | 11.7 |
| strips 2–16 | — | 0/0 · 0/0 | 19–23/1 · 14–18/2, 2 | 1–3 | 2–3 |

**The levels follow the offset as the reading predicts.** The inputs flip on a number of pixels that
grows with the offset (43, 73, 137 at one, two and four units) and stay one level until a
sixty-fourth of a pixel. The composed cell moves by its input's move times the blend's gain there:
one for Normal (two where both inputs move), up to twelve for ColorDodge, whose `1 / (1 − Cs)` is
steep for a bright source and clamped at one. For Saturation and Hue the gain is not a slope: it is
four at one unit and 48 to 93 from a thousandth of a pixel, because `SetSat` divides by
`Cmax − Cmin`, so near a neutral colour a one-level move reorders its components and the result jumps.
A strip's perturbation is smaller than one unit of `ty` (fewer input flips than `n` = 1). That is
why the page under strips reaches a gain of three and no more: few samples flip, and none where
Saturation jumps.

## 4. Decision: what the ceiling rests on

The reading holds, narrowed. **The division moves one photograph's sample by at most one level —
`strip_parallelism.rs`'s one-coverage bound, holding of the sample — and the blend function
multiplies that by its slope at the pixel.** The output has no closed-form ceiling: Table 134's
ColorDodge and ColorBurn are steep near their poles, and Table 135's functions are discontinuous at
neutral colours. So `PAST_THE_BOUND` keeps the page's ceiling as a measured figure, 83 pixels and 3
levels, and its note now says what the moved pixels are. The derived half is asserted where it can
be derived. The new test draws every photograph of the page alone on the page's own plan and holds
it to one level at each of the golden's four divisions. It also holds the four cells composited from
eight-bit rasters to their inputs. A later move of the ceiling therefore comes with a test that says
which half moved.

## 5. Calibrated both ways (trap 13)

From an export of HEAD with this diff, its own `CARGO_TARGET_DIR`:

- **Every strip below the first shifted by 1/16 px** in `ToDevice::of`: the test fails, naming the
  Darken cell's backdrop, drawn alone in 2 strips, as moved by 3 levels at (86, 397). The page
  test fails too (3 699 pixels, worst 19).
- **Only the layer Table 135's modes are composited from shifted by 1/64 px**: the test fails,
  naming the Hue cell in 2 strips as moved at (229, 397) where neither of its two photographs moved.
- **Unplanted**: both tests pass, 0.15 s and 0.98 s in the dev profile.

## 6. Cost

Nothing drawn changed. The test adds 0.15 s to `pdf-model`'s suite. One note in `raster_golden.rs`
and one sentence of `strip_parallelism.rs`'s module comment now cite this ADR.
