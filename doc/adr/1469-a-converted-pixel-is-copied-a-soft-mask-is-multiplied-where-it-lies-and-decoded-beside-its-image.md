# 1469 — A converted pixel is copied, a soft mask is multiplied where it lies and decoded beside its image

Status: accepted. Session 1317. Keeps ADR 1321's decodes ahead, ADR 1433's paired walk and ADR
1457's matte table; answers the image half of `doc/performance.md`'s slowest turn. Supersedes
nothing.
Context: ISO 32000-2 §8.6.5.5, §8.9.5.2, §11.6.5.2 and Tables 143–144; `CLAUDE.md` principle 2
and the rule on optimisations; `doc/questions/A121`; `doc/habits/measuring.md` 48, 52, 55, 59;
traps 50, 66, 73.
Code: `crates/pdf-model/src/image.rs` — `convert_three` and `convert_four` (a run answered before
the memo, the memo holding bytes), `SampleMemo<V>`, `opacity_multiplied_in_place`,
`matte_before_samples`, `EagerMask`, `samples_beside_the_mask`, `beside`, `alpha_of_mask`, and
`apply_soft_mask` taking what was made.
Tests: `image.rs`'s `a_converted_pixel_is_its_own_samples_conversion`,
`a_mask_multiplied_in_place_is_the_combination`, `beside_answers_both`; `raster_golden` held.

## 1. What was measured first, and what it said about the brief

Callgrind on `examples/callgrind_interpret`, `RAYON_NUM_THREADS=1`, functions by name; scratch
`Instant`s around each step of `decode_parts` with the thread named, removed since;
`examples/frame_budget` pinned to the eight CPUs whose `cpuinfo_max_freq` is highest, minimum of
three runs of five rounds, every arm one binary with scratch switches, removed since.

**`22060_A1_01_Plans.pdf` is four photographs, not seventy-two.** Fourteen image `XObject`s: four
2 480 × 2 630 `DCTDecode` images in one `ICCBased` space, each under a 2 480 × 2 630 `DCTDecode`
`DeviceGray` `/SMask`, and six small ones; the 72 are placements through tiling cells, answered by
`RasterCache`. ADR 1321's walk offers the five past `AHEAD_FLOOR` and leaves the first to the
interpreter, so the four photographs decode at once, one on the interpreter's thread and three on
the pool. Of 4 262 M instructions, `convert_three` was **2 143 M** (50%), `zune-jpeg` about 1 500 M
and `apply_soft_mask` 548 M.

- **The memo was not missing; its hit was the cost.** One profile, 256 distinct colours a
  photograph, 8 136 conversions for 26 M pixels — 99.97% answered by the memo — and still 82
  instructions a pixel: a hash, a probe, a key compare and three float roundings of the cached
  colour. 84% of the pixels equal the one before them.
- **The two divisions per pixel were gone** — ADR 1433 took `scale`'s out of the walk. What the
  combination still paid was a zeroed 26 MB raster per photograph, its walk, a 26 MB copy into the
  `Arc`, and its place in line: after the image's conversion and after the mask's own decode.
- **The conversion and the combination were already in the pool's task**, because a decode ahead
  is `decode_parts` whole. What was serial was inside it: per photograph, JPEG 13–20 ms, conversion
  23–33, the mask's JPEG 10–20, the combination 10–13.
- **`issue13931.pdf`** (1 145 M): the matte table 396 M, `apply_soft_mask` 281 M and `unpack` 166 M —
  the mask's `FlateDecode` samples unpacked twice, once for §11.6.5.2's inversion and once for the
  opacity. The clause fixes the order of the first: "inversion of the pre-blending shall precede the
  colour conversion". It fixes nothing about decoding the mask twice.

## 2. The levers, each alone (pinned, eight threads, turn and its `interp`, ms)

| page | all off | conversion | in place | beside | once | all |
|---|---|---|---|---|---|---|
| `22060_A1_01_Plans.pdf` | 94.42 (76.48) | 76.02 (58.64) | 82.12 (63.82) | 80.02 (64.49) | — | **58.37 (39.82)** |
| `issue13931.pdf` | 66.80 (60.59) | — | 53.11 (46.75) | 55.79 (49.44) | 54.21 (47.26) | **41.31 (35.31)** |
| `images.pdf` | 51.26 (43.81) | — | 40.66 (32.86) | 47.58 (39.17) | — | **38.55 (30.84)** |

Load 7.7–10.2. The finished tree against the tree before it, one sitting at load 2.2–2.5, pinned:
the plan 86.64 (71.26) → **52.91 (36.97)**, `issue13931` 60.94 (55.47) → **38.76 (33.85)**,
`images.pdf` 47.84 (41.01) → **35.07 (28.49)**, and the photograph of `issue12841_reduced.pdf`,
which none of the four reaches, 39.32 → 39.96; unpinned, 24 threads, 80.14 → 53.10, 64.36 →
41.28, 48.24 → 38.88 and 36.07 → 35.48. Callgrind, all off against all on: the plan 4 288 → 2 181 M, `issue13931` 1 145 →
838 M. No byte moved: `raster_golden` held.

- **A converted pixel is copied.** The memo keeps the three bytes the conversion rounded to, not
  the colour, and a pixel whose three samples equal the one before it is answered from that one by
  one masked compare of a word. The bytes are the conversion of the same tuple either way. 2 143 →
  455 M on the plan; `convert_four` takes the same two steps.
- **A soft mask is multiplied where it lies.** `decode_parts` has just made the raster and nothing
  else holds it, so `Arc::get_mut` answers and the alpha bytes are rewritten in place, a word at a
  time, divided by `band_pixels` across the pool: no second raster, no zeroing, no copy. Same rounding
  as `combine_on_the_finer_grid`, held by a test against it. `apply_soft_mask` 548 → 170 M (plan),
  281 → 83 M (`issue13931`). A pair on two grids, or a raster held elsewhere, takes
  `combine_on_the_finer_grid` as before.
- **The mask is decoded beside its image.** `eager_soft_mask` is a function of the dictionary
  alone; where its grid is the one `decode_parts` read, the image is at least `PARALLEL_PIXELS` and
  the codec is not `JPXDecode`, it is started on the pool before the image's samples. `beside` takes
  it back if no thread has begun it, so it never waits on work not started. A route that then does
  not reach `apply_soft_mask` has decoded a mask for nothing; that is the whole cost of a wrong guess.
- **A `/Matte`'s mask is decoded once.** Where the grids agree, the decode the inversion reads is
  the eager route's own call on the same stream, so its answer is handed on. `unpack` 166 → 83 M.

## 3. The pool's width, and what was not built

`RAYON_NUM_THREADS` 4, 8, 16, the plan with all four levers: pinned 63.46, 59.35, 61.03; unpinned
69.87, 57.95, 56.16. Past eight the width is not what limits the page; nothing was changed. A
lazily filled exact table for an eight-bit three-component tuple (2^24 keys) was not built: the memo
already answered 99.97%, and the cost was the hit, which the run and the bytes took.

## 4. What is left

The plan's 37 ms of interpretation is now eight `DCTDecode` frames with no restart interval —
`decode_mcu_block` 625 M, one thread a frame, all eight cores busy (ADR 1457 section 3 is why they
are not cut) — and `convert_three`'s 455 M. `issue13931`'s is the matte table (396 M) and the mask's
widening into four-byte pixels, read back one byte a pixel (83 M): a mask decoded as a one-byte plane
would save most of the second.
