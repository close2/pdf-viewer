# 1493 — A zoom step fills the texels its window samples, a translucent band is read off premultiplied column sums, and a reduced row is premultiplied on the thread that made it

Status: accepted. Session 1329. Answers `doc/QUORRA_FEEDBACK.md` section 52 ask 2 on this side
(section 64);
keeps ADR 0089's per-placement reduction, ADR 1433's row division and ADR 1457's opaque column
sums. Supersedes nothing.
Context: ISO 32000-2 §8.9.5, §10.7.4; `CLAUDE.md` principle 2 and the rule on optimisations;
`doc/questions/A121`; ADRs 0089, 1289, 1433, 1457, 1472, 1481; `doc/habits/measuring.md` 55,
57, 59; traps 50, 66, 73.
Code: `raster/crates/raster-gpu/src/device/textures.rs` (`TexelRect`, `TexelRect::sampled`,
`FillRecord`, `FillRecord::claim`, `PaintTexture`, `paint_texture`, `write_texels`,
`write_samples`, `opaque`, `premultiply_in_place`), `raster/crates/raster-gpu/src/device/resident.rs`
(`fill_sampled`, `ensure_paint_textures`, `release`), `raster/crates/raster-gpu/src/raster/reduce.rs`
(`area_averaged_cells`, `reduce_row`, `translucent_row`), `raster/crates/raster-gpu/src/device.rs`
(two field types), `raster/crates/raster-gpu/src/device/rare.rs` (one lookup).
Tests: `reduce.rs`'s `a_window_of_cells_is_the_whole_grid_cropped` and
`a_translucent_row_is_the_per_block_arithmetic`; `textures.rs`'s
`a_footprint_names_the_texels_it_can_read`, `a_claim_owes_only_what_no_earlier_claim_filled` and
`premultiplying_is_round_to_nearest_at_every_pair`; `tests/an_image_is_filled_where_it_is_sampled.rs`
(calibrated: with the margin at zero and squares of one texel it fails, 779 bytes on the magnified
linear view).

## 1. What a step's transfer was

`frame_budget`'s step places the page at twice the magnification in a 1 600 × 1 000 window on the
compute lane. A probe in `ensure_paint_textures` (removed) timed each image:

| page | what the step did | reduce | premultiply | write |
|---|---|---|---|---|
| photograph, 5 280 × 3 792 | no reduction at 2×: the **full samples, 80 MB**, uploaded whole | — | 2.25 (opaque test) | 12.60 |
| Plans | four 2 480 × 2 630 frames reduced 3:1, one after another | 4 × 2.5 | 4 × 1.2 | 4 × 0.2 |
| `issue13931.pdf` | one 2 996 × 4 256 frame reduced 3:1 | 6.3 | 3.7 | 0.6 |
| `images.pdf` | five frames reduced at new factors | 6.7 | 3.1 | 2.1 |

So yes: **every step at a new factor reduced the whole image again from the full samples**, the
reduced raster was premultiplied on one thread after the reduction, and a texture was uploaded
whole although the window showed a part of it — on the photograph a fifth (2 640 × 1 896 cells at
1×, a 1 600 × 1 000 window). The turn's transfer on the photograph was the same shape: a 2:1
reduction of all of it for a window showing a third.

## 2. The levers, each alone

One binary with scratch switches, pinned to the eight performance cores, the arms interleaved run
by run so that a neighbour's load falls on all of them, minimum of five runs of five rounds, load
4.6–7.2; milliseconds, step budget (transfer):

| page | all off | (a) alone | (e) alone | (f) alone | all three |
|---|---|---|---|---|---|
| photograph | 14.71 (11.36) | 4.12 (1.51) | 15.10 (11.86) | 15.02 (11.62) | **3.79 (1.52)** |
| Plans | 27.55 (17.68) | 21.42 (11.67) | 23.50 (13.41) | 23.40 (13.87) | **17.36 (7.62)** |
| `issue13931.pdf` | 9.29 (7.26) | 8.97 (6.99) | 8.00 (6.06) | 8.43 (6.41) | **7.09 (5.04)** |
| `images.pdf` | 14.28 (8.01) | 14.13 (7.99) | 12.74 (6.37) | 13.32 (7.05) | **11.68 (5.27)** |

(e) and (f) move nothing on the photograph, which is opaque and unreduced at 2×; (a) moves little
on the two pages whose images the 2× window shows almost whole. Each pays where its subject is,
and the three together are the least on every page. Earlier sittings that ran the arms one after
another rather than interleaved ranked `images.pdf` the other way round at load 9–11, and are not
used.

**(a) Built: a texture is filled where a frame samples it.** The texture is made at the full size
of its grid — the image's own, or a reduction's — so `image.wgsl`'s coordinate for every pixel is
the one a whole texture is asked (`textureDimensions` is unchanged), and `FillRecord` records which
256-texel squares hold their bytes. Each image op's `dest` corners are carried through its device →
texel transform, bounded, widened by two texels — the linear filter's neighbour and the shader's
`f32` evaluation against this `f64` one — and clamped as the shader clamps. Only unfilled squares
are produced: the image's own rows written straight out of its bytes where they are opaque, and a
reduction's cells reduced for that window alone (`area_averaged_cells`). A cell reads only its own
block, so a window is the whole grid cropped; an opaque band within the window may take the column
sums where the whole width would not, and ADR 1457's proof makes that the same byte. The device
memory is unchanged — the texture was always the grid's full size — and a scroll owes the strip it
uncovers. Released images now take their reductions with them (`release` left them for the
device's life).

**(b) Declined: deriving the next scale's reduction from the last one.** Not exact. A mean of
rounded means is not the rounded mean: at factor 3, opaque samples `0 1 1 | 0 0 0` reduce to 1
(⅔ rounded) and 0, and those two to 1 (½ rounded up), where the factor-6 cell of all six is ⅓
rounded, 0. Proportional
bands at `⌈S/f⌉` and `⌈S/2f⌉` cells do not nest unless the first count is twice the second. And
`frame_budget`'s step zooms *in*, to a finer grid no coarser raster can produce. Exact derivation
would need the sums kept rather than the bytes — four times the memory for a case (a) has made
small.

**(c) Declined: a mip chain sampled at the nearest level.** The step frame is not always followed
by an exact one: with `--supersample 1` there is no sharp pass, and with it the pass is declined
without a word where `sharp_pass_cost` exceeds `SHARP_STALL_BUDGET` (ADR 1289). In both the step
frame is the frame left on the screen, and a mip-sampled image there is a picture the oracle's
§10.7.4 reduction does not make, kept — the "plausible lie" `doc/questions/A121` rules out for an
early present. No gate would see it either: `render-raster --test corpus` compares a whole page on
a fresh device, never a step. It would also need `image.wgsl` and a mip-level pass, and after (a)
the photograph's step transfer it was asked for is 1.3 ms.

**(d) Measured, and answered by (a): the full-size texture.** The step did upload a texture the
turn had not: the turn drew the photograph through its 2:1 reduction, and the 2× step through the
samples themselves. The full texture is already kept for the device's life once made, so a second
step at native size uploads nothing new; what (a) removes is uploading all of it for a window that
shows a fifth. Sampling a retained full texture at a reduced scale instead of the reduction is not
§10.7.4's area average (ADRs 0025, 0089) and is (c)'s case again.

**(e) Built: a reduced row is premultiplied on the thread that made it.** `area_averaged_cells`
hands each finished row to a `finish` step on its own thread, and the device premultiplies there
(a row that is all opaque is left alone, as before for a whole texture). Same function, same bytes,
divided among `Options::encode_threads`. A plain-arithmetic rewrite of the premultiplication was
measured first and moved nothing (2.10 → 1.99 ms, inside the spread): without SSE4.1 the baseline
target cannot multiply `u32` lanes, so it stays scalar; it was not kept.

**(f) Built: a translucent band is read off premultiplied column sums.** A band that is not opaque
took `average_block` per cell. It now sums `c·a` for each component and `a` down each column once
and adds columns per cell; integer addition is order-free, so each cell divides the same sums as
`average_block` and is its byte, bounded under 2^28 by ADR 1457's `SMALL_BLOCK`. Writing the
band's first row over the sums rather than clearing them took the clear (a fifth of the function on
a 3:1 band) away. Callgrind, the whole `frame_budget` run (three reductions of each image):
`issue13931.pdf`'s reduction and its clears 1 079 → 700 M instructions; before the clear was
removed, Plans 2 311 → 1 800 M and `images.pdf` 1 050 → 886 M.

**The CPU oracle keeps its originals.** `pdf_render::Image::area_averaged` is the reduction the
cross-backend gates hold raster to; (a) and (f) are raster's own constructions of the same bytes,
each held to the originals by a test rather than mirrored statement for statement, and `reduce.rs`'s
module comment says which.

## 3. The whole

`frame_budget`, the tree before (at the batch's open) against the tree after, one sitting, two
`md5sum`-distinct binaries, pinned, minimum of three runs of five rounds interleaved page by page,
load 3.0–3.4; milliseconds, budget (transfer), bytes the frame uploaded:

| page | step before | step after | bytes | turn before | turn after |
|---|---|---|---|---|---|
| photograph | 13.72 (10.81) | **3.24 (1.30)** | 80 087 104 → 6 553 664 | 46.28 (13.27) | **31.88 (1.86)** |
| Plans | 24.44 (15.68) | **15.31 (6.80)** | 16 982 600 → 13 595 208 | 54.76 (9.19) | 52.50 (8.05) |
| `issue13931.pdf` | 9.14 (7.81) | **6.17 (4.95)** | unchanged | 25.73 | 25.58 |
| `images.pdf` | 11.52 (7.28) | **9.95 (4.95)** | unchanged | 34.92 | 36.17 |

The text, mesh, stroke, `issue19802.pdf` and Type 3 rows moved inside their spread. The "after"
binary also carries what its siblings changed in raster beside this round, so the lever table in
section 2, one binary with switches, is the attribution; this is the whole as it will merge. The
photograph's turn gains because `frame_budget`'s turn places the page at one pixel a point, where the
window shows a third of the photograph; a window that shows a page whole still reduces all of it.
`raster_golden` held 974 and moved 0 (the CPU backend's bytes, which this does not touch);
`render-raster --test corpus` at 1×: 966 agree, 0 differ, 1 refused, 7 not comparable, 1-vs-N 0,
peak 5.10 GiB.

## 4. What is left

On `issue13931.pdf` and `images.pdf` the window at 2× shows most of each image, so (a) has little
to take and the step's transfer is the reduction itself — about 17 instructions a sample on the
translucent path, scalar on the baseline target. Plans' step is now as much `encode` as transfer.
Section 52's device route stays priced as it was: on a step that draws the samples themselves it
had nothing to take, and that upload is now the window's.
